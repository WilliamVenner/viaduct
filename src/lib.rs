//! Viaduct is a library for establishing a duplex communication channel between a parent and child process, using unnamed pipes.
//!
//! # Example
//!
//! ## Shared library
//!
//! ```no_run
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! pub enum ExampleRpc {
//!     Cow,
//!     Pig,
//!     Horse
//! }
//!
//! #[derive(Serialize, Deserialize, PartialEq, Eq)]
//! pub struct FrontflipError;
//!
//! #[derive(Serialize, Deserialize, PartialEq, Eq)]
//! pub struct BackflipError;
//! ```
//!
//! ## Parent process
//!
//! ```no_run
//! # use viaduct::doctest::*;
//! let child = std::process::Command::new("child.exe");
//! let ((tx, rx), mut child) = viaduct::ViaductParent::new(child).unwrap().build().unwrap();
//!
//! std::thread::spawn(move || {
//!     rx.run(
//!         |rpc: ExampleRpc| match rpc {
//!             ExampleRpc::Cow => println!("Moo"),
//!             ExampleRpc::Pig => println!("Oink"),
//!             ExampleRpc::Horse => println!("Neigh"),
//!         },
//!
//!         |request: ExampleRequest, tx| match request {
//!             ExampleRequest::DoAFrontflip => {
//!                 println!("Doing a frontflip!");
//!                 tx.respond(Ok::<_, FrontflipError>(())).unwrap();
//!             },
//!
//!             ExampleRequest::DoABackflip => {
//!                 println!("Doing a backflip!");
//!                 tx.respond(Ok::<_, BackflipError>(())).unwrap();
//!             },
//!         },
//!     ).unwrap();
//! });
//!
//! tx.rpc(ExampleRpc::Cow).unwrap();
//! tx.rpc(ExampleRpc::Pig).unwrap();
//! tx.rpc(ExampleRpc::Horse).unwrap();
//!
//! let response: Result<(), FrontflipError> = tx.request(ExampleRequest::DoAFrontflip).unwrap();
//! assert_eq!(response, Ok(()));
//! ```
//!
//! ## Child process
//!
//! ```no_run
//! # use viaduct::doctest::*;
//! let (tx, rx) = unsafe { viaduct::ViaductChild::new().build() }.unwrap();
//!
//! std::thread::spawn(move || {
//!     rx.run(
//!         |rpc: ExampleRpc| match rpc {
//!             ExampleRpc::Cow => println!("Moo"),
//!             ExampleRpc::Pig => println!("Oink"),
//!             ExampleRpc::Horse => println!("Neigh"),
//!         },
//!
//!         |request: ExampleRequest, tx| match request {
//!             ExampleRequest::DoAFrontflip => {
//!                 println!("Doing a frontflip!");
//!                 tx.respond(Ok::<_, FrontflipError>(())).unwrap();
//!             },
//!
//!             ExampleRequest::DoABackflip => {
//!                 println!("Doing a backflip!");
//!                 tx.respond(Ok::<_, BackflipError>(())).unwrap();
//!             },
//!         },
//!     ).unwrap();
//! });
//!
//! tx.rpc(ExampleRpc::Horse).unwrap();
//! tx.rpc(ExampleRpc::Pig).unwrap();
//! tx.rpc(ExampleRpc::Cow).unwrap();
//!
//! let response: Result<(), BackflipError> = tx.request(ExampleRequest::DoABackflip).unwrap();
//! assert_eq!(response, Ok(()));
//! ```
//!
//! # Use Cases
//!
//! Viaduct was designed for separating user interface from application logic in a cross-platform manner.
//!
//! For example, an application may want to run a GUI in a separate process from the application logic, for modularity or performance reasons.
//!
//! Viaduct allows for applications like this to communicate between these processes in a natural way, without having to manually implement IPC machinery & synchronization.
//!
//! # Usage
//!
//! ## Serialization
//!
//! Viaduct currently supports serialization and deserialization of data using [`bytemuck`](https://docs.rs/bytemuck) (default), [`bincode`](https://docs.rs/bincode) or [`speedy`](https://docs.rs/speedy) at your choice, using the respective Cargo feature flags.
//!
//! You can also manually implement the [`ViaductSerialize`] and [`ViaductDeserialize`] traits.
//!
//! ## Initializing a viaduct
//!
//! A viaduct is started by calling [`ViaductParent::new`] as the parent process, which will spawn your child process.
//!
//! Your child process should then call [`ViaductChild::new`], [`ViaductChild::new_with_args_os`] or [`ViaductChild::new_with_args`] (see CAVEAT below) to bridge the connection between the parent and child.
//!
//! Then, you are ready to start...
//!
//! ## Passing data
//!
//! Viaduct has two modes of operation: RPCs and Requests/Responses.
//!
//! RPCs are one-way messages, and are useful for sending notifications to the other process.
//!
//! Requests/Responses are two-way messages, and are useful for sending requests to the other process and receiving data as a response.
//!
//! Requests will block any other thread trying to send requests and RPCs through the viaduct, until a response is received.
//!
//! ## CAVEAT: Don't use [`std::env::args_os`] or [`std::env::args`] in your child process!
//!
//! The child process should not use `args_os` or `args` to get its arguments, as these will contain data Viaduct needs to pass to the child process.
//!
//! Instead, use the argument iterator provided by [`ViaductChild::new_with_args_os`] or [`ViaductChild::new_with_args`] for `args_os` and `args` respectively.

#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]
#![cfg_attr(ci_test, deny(warnings))]

#[cfg(not(any(unix, windows)))]
compile_error!("Unsupported platform");

use interprocess::unnamed_pipe::{UnnamedPipeReader, UnnamedPipeWriter};
use parking_lot::{Condvar, Mutex};
use std::{
	ffi::{OsStr, OsString},
	io::{Read, Write},
	marker::PhantomData,
	num::NonZeroU64,
	process::{Child, Command},
	sync::Arc,
};

mod chan;
pub use chan::*;

mod serde;
pub use self::serde::{Never, ViaductDeserialize, ViaductSerialize};

mod os;
use os::RawPipe;

mod reaper;
use reaper::{DroppablePipe, ReaperCallbackFn};

mod debugs;

#[doc(hidden)]
pub mod doctest;

fn verify_channel<R, F: FnOnce() -> Result<R, std::io::Error>>(
	tx: &mut UnnamedPipeWriter,
	rx: &mut UnnamedPipeReader,
	ready: F,
) -> Result<R, std::io::Error> {
	tx.write_all(chan::HELLO)?;
	tx.write_all(&u16::to_ne_bytes(0x0102_u16))?;
	tx.write_all(&u128::to_ne_bytes(core::mem::size_of::<usize>() as _))?;

	let ready = ready()?;

	let mut hello = [0u8; chan::HELLO.len()];
	rx.read_exact(&mut hello)?;
	if hello != chan::HELLO {
		return Err(std::io::Error::new(
			std::io::ErrorKind::BrokenPipe,
			"Child process didn't respond with hello message",
		));
	}

	let mut endianness = [0u8; core::mem::size_of::<u16>()];
	rx.read_exact(&mut endianness)?;
	let endianness = u16::from_ne_bytes(endianness);
	if endianness != 0x0102_u16 {
		return Err(std::io::Error::new(
			std::io::ErrorKind::Unsupported,
			"Child process is using a different endianness",
		));
	}

	let mut usize_size = [0u8; core::mem::size_of::<u128>()];
	rx.read_exact(&mut usize_size)?;
	if u128::from_ne_bytes(usize_size) != core::mem::size_of::<usize>() as u128 {
		return Err(std::io::Error::new(
			std::io::ErrorKind::Unsupported,
			"Child process is running on a different architecture",
		));
	}

	Ok(ready)
}

fn channel<RpcTx, RequestTx, RpcRx, RequestRx>(tx: UnnamedPipeWriter, rx: UnnamedPipeReader) -> Viaduct<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize,
{
	let tx = ViaductTx(Arc::new(ViaductTxInner {
		response_condvar: Condvar::new(),
		response: Mutex::new(ViaductResponseState::default()),
		state: Mutex::new(ViaductTxState::new(tx)),
	}));
	let rx = ViaductRx {
		buf: Vec::new(),
		tx: tx.clone(),
		rx,
		_phantom: Default::default(),
	};
	(tx, rx)
}

/// Interface for creating a viaduct on the **PARENT** process.
///
/// `RpcTx` is the type sent to the child process for RPC. In the child process' code, this would be `RpcRx`
///
/// `RpcRx` is the type received from the child process for RPC. In the child process' code, this would be `RpcTx`
///
/// `RequestTx` is the type sent to the child process for requests. In the child process' code, this would be `RequestRx`
///
/// `RequestRx` is the type received from the child process for requests. In the child process' code, this would be `RequestTx`
pub struct ViaductParent<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize,
{
	command: Command,
	tx: ViaductTx<RpcTx, RequestTx, RpcRx, RequestRx>,
	rx: ViaductRx<RpcTx, RequestTx, RpcRx, RequestRx>,
	_reaper_rx: DroppablePipe<UnnamedPipeReader>,
	reaper_tx: DroppablePipe<UnnamedPipeWriter>,
	with_reaper: Option<ReaperCallbackFn>,
}
impl<RpcTx, RequestTx, RpcRx, RequestRx> ViaductParent<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize,
{
	/// Initializes the viaduct in the parent process.
	///
	/// # Panics
	///
	/// This function will panic if the [`Command`](std::process::Command) has arguments set.
	///
	/// You can set command arguments using the [`ViaductParent::arg`] and [`ViaductParent::args`] methods.
	pub fn new(mut command: Command) -> Result<Self, std::io::Error> {
		if command.get_args().next().is_some() {
			panic!("Command must not have any arguments - to add arguments to your command please use the `arg` method and `args` method of this builder");
		}

		let (child_w, child_r) = interprocess::unnamed_pipe::pipe()?;
		let (parent_w, parent_r) = interprocess::unnamed_pipe::pipe()?;

		let (reaper_tx, reaper_rx) = interprocess::unnamed_pipe::pipe()?;
		let (reaper_tx, reaper_rx) = (DroppablePipe::new(reaper_tx), DroppablePipe::new(reaper_rx));

		command.arg("PIPER_START");
		command.args(&[
			(parent_w.raw() as usize as u64).to_string(),
			(child_r.raw() as usize as u64).to_string(),
			(reaper_tx.as_raw() as usize as u64).to_string(),
			(reaper_rx.as_raw() as usize as u64).to_string(),
		]);

		let (tx, rx) = channel(child_w, parent_r);

		Ok(Self {
			command,
			tx,
			rx,
			with_reaper: None,
			reaper_tx,
			_reaper_rx: reaper_rx,
		})
	}

	/// Adds an argument to the [`Command`](std::process::Command)
	pub fn arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
		self.command.arg(arg.as_ref());
		self
	}

	/// Adds a group of arguments to the [`Command`](std::process::Command)
	pub fn args<I, S>(mut self, args: I) -> Self
	where
		I: IntoIterator<Item = S>,
		S: AsRef<OsStr>,
	{
		self.command.args(args);
		self
	}

	#[inline]
	/// Whether to spawn a reaper thread or not.
	///
	/// A reaper thread will occasionally check whether the child process has been killed and call your `callback` if it has.
	///
	/// This allows you to gracefully handle the child process being killed.
	pub fn with_reaper<F: FnOnce() + Send + 'static>(mut self, callback: F) -> Self {
		self.with_reaper = Some(Box::new(callback));
		self
	}

	/// Spawns the child process and returns it along with a [`Viaduct`](crate::Viaduct).
	#[allow(clippy::type_complexity)]
	pub fn build(mut self) -> Result<(Viaduct<RpcTx, RequestTx, RpcRx, RequestRx>, Child), std::io::Error> {
		struct KillHandle(Option<Child>);
		impl Drop for KillHandle {
			#[inline]
			fn drop(&mut self) {
				if let Some(child) = &mut self.0 {
					child.kill().ok();
				}
			}
		}

		let mut child = verify_channel(&mut self.tx.0.state.lock().tx, &mut self.rx.rx, move || {
			Ok(KillHandle(Some(self.command.spawn()?)))
		})?;

		let child = child.0.take().unwrap();

		if let Some(callback) = self.with_reaper {
			unsafe { reaper::parent(self.reaper_tx, callback) };
		} else {
			std::mem::forget(self.reaper_tx);
		}

		Ok(((self.tx, self.rx), child))
	}
}

/// Interface for creating a viaduct on the **CHILD** process.
///
/// `RpcTx` is the type sent to the parent process for RPC. In the parent process' code, this would be `RpcRx`
///
/// `RpcRx` is the type received from the parent process for RPC. In the parent process' code, this would be `RpcTx`
///
/// `RequestTx` is the type sent to the parent process for requests. In the parent process' code, this would be `RequestRx`
///
/// `RequestRx` is the type received from the parent process for requests. In the parent process' code, this would be `RequestTx`
pub struct ViaductChild<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize,
{
	with_reaper: Option<ReaperCallbackFn>,
	_phantom: PhantomData<(RpcTx, RequestTx, RpcRx, RequestRx)>,
}
impl<RpcTx, RequestTx, RpcRx, RequestRx> ViaductChild<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize,
{
	#[inline]
	#[allow(clippy::new_without_default)]
	/// Starts building a new viaduct in the child process.
	pub fn new() -> Self {
		Self {
			with_reaper: None,
			_phantom: Default::default(),
		}
	}

	#[inline]
	/// Whether to spawn a reaper thread or not.
	///
	/// A reaper thread will occasionally check whether the parent process has been killed and call your `callback` if it has.
	///
	/// This allows you to gracefully handle the parent process being killed.
	pub fn with_reaper<F: FnOnce() + Send + 'static>(mut self, callback: F) -> Self {
		self.with_reaper = Some(Box::new(callback));
		self
	}

	/// Initializes a viaduct in the child process.
	///
	/// Returns the viaduct.
	///
	/// # Safety
	///
	/// Undefined behaviour can result from manipulating the program's arguments in a way that disrupts Viaduct's handle exchange.
	pub unsafe fn build(self) -> Result<Viaduct<RpcTx, RequestTx, RpcRx, RequestRx>, std::io::Error> {
		let mut args = std::env::args_os();
		{
			let sig = OsStr::new("PIPER_START");
			let mut sig_found = false;
			for arg in args.by_ref() {
				if arg == sig {
					sig_found = true;
					break;
				}
			}
			if !sig_found {
				return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Could not find pipe handles"));
			}
		}

		let (parent_w, child_r, reaper_tx, reaper_rx) = match args
			.next()
			.and_then(|arg| Some((arg, args.next()?, args.next()?, args.next()?)))
			.and_then(|pipes| {
				Some((
					pipes.0.to_str()?.parse::<NonZeroU64>().ok()?,
					pipes.1.to_str()?.parse::<NonZeroU64>().ok()?,
					pipes.2.to_str()?.parse::<NonZeroU64>().ok()?,
					pipes.3.to_str()?.parse::<NonZeroU64>().ok()?,
				))
			}) {
			Some(pipes) => pipes,
			_ => return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Could not parse pipe handles")),
		};

		unsafe { Self::child_handshake(parent_w, child_r, reaper_tx, reaper_rx, self.with_reaper) }
	}

	/// Initializes a viaduct in the child process.
	///
	/// Returns the viaduct and the process arguments.
	///
	/// # Safety
	///
	/// Undefined behaviour can result from manipulating the program's arguments in a way that disrupts Viaduct's handle exchange.
	pub unsafe fn build_with_args_os(self) -> Result<(Viaduct<RpcTx, RequestTx, RpcRx, RequestRx>, impl Iterator<Item = OsString>), std::io::Error> {
		let mut args = std::env::args_os();
		let mut buffer = Vec::with_capacity(1);

		{
			let sig = OsStr::new("PIPER_START");
			let mut sig_found = false;
			for arg in args.by_ref() {
				if arg == sig {
					sig_found = true;
					break;
				}
				buffer.push(arg);
			}
			if !sig_found {
				return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Could not find pipe handles"));
			}
		}

		let (parent_w, child_r, reaper_tx, reaper_rx) = match args
			.next()
			.and_then(|arg| Some((arg, args.next()?, args.next()?, args.next()?)))
			.and_then(|pipes| {
				Some((
					pipes.0.to_str()?.parse::<NonZeroU64>().ok()?,
					pipes.1.to_str()?.parse::<NonZeroU64>().ok()?,
					pipes.2.to_str()?.parse::<NonZeroU64>().ok()?,
					pipes.3.to_str()?.parse::<NonZeroU64>().ok()?,
				))
			}) {
			Some(pipes) => pipes,
			_ => return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Could not parse pipe handles")),
		};

		Ok((
			unsafe { Self::child_handshake(parent_w, child_r, reaper_tx, reaper_rx, self.with_reaper)? },
			buffer.into_iter().chain(args),
		))
	}

	/// Initializes a viaduct in the child process.
	///
	/// Returns the viaduct and the process arguments.
	///
	/// # Panics
	///
	/// This function will panic if any of the program arguments are not valid Unicode.
	///
	/// # Safety
	///
	/// Undefined behaviour can result from manipulating the program's arguments in a way that disrupts Viaduct's handle exchange.
	pub unsafe fn build_with_args(self) -> Result<(Viaduct<RpcTx, RequestTx, RpcRx, RequestRx>, impl Iterator<Item = String>), std::io::Error> {
		let mut args = std::env::args();
		let mut buffer = Vec::with_capacity(1);

		{
			let mut sig_found = false;
			for arg in args.by_ref() {
				if arg == "PIPER_START" {
					sig_found = true;
					break;
				}
				buffer.push(arg);
			}
			if !sig_found {
				return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Could not find pipe handles"));
			}
		}

		let (parent_w, child_r, reaper_tx, reaper_rx) = match args
			.next()
			.and_then(|arg| Some((arg, args.next()?, args.next()?, args.next()?)))
			.and_then(|pipes| {
				Some((
					pipes.0.parse::<NonZeroU64>().ok()?,
					pipes.1.parse::<NonZeroU64>().ok()?,
					pipes.2.parse::<NonZeroU64>().ok()?,
					pipes.3.parse::<NonZeroU64>().ok()?,
				))
			}) {
			Some(pipes) => pipes,
			_ => return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Could not parse pipe handles")),
		};

		Ok((
			unsafe { Self::child_handshake(parent_w, child_r, reaper_tx, reaper_rx, self.with_reaper)? },
			buffer.into_iter().chain(args),
		))
	}

	unsafe fn child_handshake(
		parent_w: NonZeroU64,
		child_r: NonZeroU64,
		reaper_tx: NonZeroU64,
		reaper_rx: NonZeroU64,
		with_reaper: Option<ReaperCallbackFn>,
	) -> Result<Viaduct<RpcTx, RequestTx, RpcRx, RequestRx>, std::io::Error> {
		let parent_w = unsafe { UnnamedPipeWriter::from_raw(parent_w.get() as usize as _) };
		let child_r = unsafe { UnnamedPipeReader::from_raw(child_r.get() as usize as _) };
		let (tx, mut rx) = channel(parent_w, child_r);

		let reaper_tx = DroppablePipe::new(unsafe { UnnamedPipeWriter::from_raw(reaper_tx.get() as usize as _) });
		let reaper_rx = DroppablePipe::new(unsafe { UnnamedPipeReader::from_raw(reaper_rx.get() as usize as _) });

		// Immediately drop the writer side of the reaper pipe pair
		// This closes the handle that the child process inherited
		drop(reaper_tx);

		// Verify the channel is OK
		verify_channel(&mut tx.0.state.lock().tx, &mut rx.rx, || Ok(()))?;

		// Start the reaper thread
		if let Some(callback) = with_reaper {
			unsafe { reaper::child(reaper_rx, callback) };
		} else {
			std::mem::forget(reaper_rx);
		}

		Ok((tx, rx))
	}
}
