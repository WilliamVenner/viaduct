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
//! # use viaduct::test::*;
//! let child = std::process::Command::new("child.exe");
//! let ((tx, rx), mut child) = viaduct::ViaductBuilder::parent(child).unwrap().build().unwrap();
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
//!                 tx.respond(Ok::<_, FrontflipError>(()))
//!             },
//!
//!             ExampleRequest::DoABackflip => {
//!                 println!("Doing a backflip!");
//!                 tx.respond(Ok::<_, BackflipError>(()))
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
//! # use viaduct::test::*;
//! let (tx, rx) = unsafe { viaduct::ViaductBuilder::child() }.unwrap();
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
//!                 tx.respond(Ok::<_, FrontflipError>(()))
//!             },
//!
//!             ExampleRequest::DoABackflip => {
//!                 println!("Doing a backflip!");
//!                 tx.respond(Ok::<_, BackflipError>(()))
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
//! Viaduct currently supports serialization and deserialization of data using [`bincode`](https://docs.rs/bincode) or [`speedy`](https://docs.rs/speedy) at your choice, using the respective Cargo feature flags.
//!
//! You can also manually implement the [`Pipeable`] trait.
//!
//! ## Initializing a viaduct
//!
//! A viaduct is initialized by calling [`ViaductBuilder::parent`] as the parent process, which will spawn your child process.
//!
//! Your child process should then call [`ViaductBuilder::child`], [`ViaductBuilder::child_with_args_os`], [`ViaductBuilder::child_with_args`] (see CAVEAT below) to bridge the connection between the parent and child.
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
//! Instead, use the argument iterator provided by [`ViaductBuilder::child_with_args_os`] or [`ViaductBuilder::child_with_args`] for `args_os` and `args` respectively.

#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

mod chan;
pub use chan::*;

mod serde;
pub use self::serde::Pipeable;

mod os;
use os::RawPipe;

#[doc(hidden)]
pub mod test;

use interprocess::unnamed_pipe::{UnnamedPipeReader, UnnamedPipeWriter};
use parking_lot::{Mutex, Condvar};
use std::{
	ffi::{OsStr, OsString},
	io::{Read, Write},
	process::{Child, Command},
	sync::Arc,
};

#[derive(Clone, Copy, Debug)]
/// The error returned when a child viaduct is started on an independent process, or the connection is lost or unrecognised whilst the viaduct is initializing.
pub struct ChildError;
impl std::fmt::Display for ChildError {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Child process was not launched by a parent")
	}
}
impl std::error::Error for ChildError {}

/// Interface for creating a viaduct.
pub struct ViaductBuilder<Rpc, Request>
where
	Rpc: Pipeable,
	Request: Pipeable
{
	command: Command,
	tx: ViaductTx<Rpc, Request>,
	rx: ViaductRx<Rpc, Request>,
}
impl<Rpc, Request> ViaductBuilder<Rpc, Request>
where
	Rpc: Pipeable,
	Request: Pipeable
{
	/// Initializes a viaduct in the child process.
	///
	/// Returns the viaduct.
	///
	/// # Safety
	///
	/// Undefined behaviour can result from:
	///
	/// * Manipulating the program's arguments in a way that disrupts Viaduct's handle exchange.
	/// * Mixing 32-bit and 64-bit parent and child
	pub unsafe fn child() -> Result<Viaduct<Rpc, Request>, ChildError> {
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
				return Err(ChildError);
			}
		}

		let (parent_w, child_r) = match args
			.next()
			.and_then(|arg| Some((arg, args.next()?)))
			.and_then(|pipes| Some((pipes.0.to_str()?.parse::<u64>().ok()?, pipes.1.to_str()?.parse::<u64>().ok()?)))
		{
			Some(pipes) => pipes,
			_ => return Err(ChildError),
		};

		Ok(unsafe { Self::child_handshake(parent_w, child_r)? })
	}

	/// Initializes a viaduct in the child process.
	///
	/// Returns the viaduct and the process arguments.
	///
	/// # Safety
	///
	/// Undefined behaviour can result from:
	///
	/// * Manipulating the program's arguments in a way that disrupts Viaduct's handle exchange.
	/// * Mixing 32-bit and 64-bit parent and child
	pub unsafe fn child_with_args_os() -> Result<(Viaduct<Rpc, Request>, impl Iterator<Item = OsString>), ChildError> {
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
				return Err(ChildError);
			}
		}

		let (parent_w, child_r) = match args
			.next()
			.and_then(|arg| Some((arg, args.next()?)))
			.and_then(|pipes| Some((pipes.0.to_str()?.parse::<u64>().ok()?, pipes.1.to_str()?.parse::<u64>().ok()?)))
		{
			Some(pipes) => pipes,
			_ => return Err(ChildError),
		};

		Ok((unsafe { Self::child_handshake(parent_w, child_r)? }, buffer.into_iter().chain(args)))
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
	/// Undefined behaviour can result from:
	///
	/// * Manipulating the program's arguments in a way that disrupts Viaduct's handle exchange.
	/// * Mixing 32-bit and 64-bit parent and child
	pub unsafe fn child_with_args() -> Result<(Viaduct<Rpc, Request>, impl Iterator<Item = String>), ChildError> {
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
				return Err(ChildError);
			}
		}

		let (parent_w, child_r) = match args
			.next()
			.and_then(|arg| Some((arg, args.next()?)))
			.and_then(|pipes| Some((pipes.0.parse::<u64>().ok()?, pipes.1.parse::<u64>().ok()?)))
		{
			Some(pipes) => pipes,
			_ => return Err(ChildError),
		};

		Ok((unsafe { Self::child_handshake(parent_w, child_r)? }, buffer.into_iter().chain(args)))
	}

	/// Initializes the viaduct in the parent process.
	///
	/// # Panics
	///
	/// This function will panic if the [`Command`](std::process::Command) has arguments set.
	///
	/// You can set command arguments using the [`ViaductBuilder::arg`] and [`ViaductBuilder::args`] methods.
	pub fn parent(mut command: Command) -> Result<Self, std::io::Error> {
		if command.get_args().next().is_some() {
			panic!("Command must not have any arguments - to add arguments to your command please use the `arg` method and `args` method of this builder");
		}

		let (child_w, parent_r) = interprocess::unnamed_pipe::pipe()?;
		let (parent_w, child_r) = interprocess::unnamed_pipe::pipe()?;

		command.arg("PIPER_START");
		command.args(&[(parent_w.raw() as u64).to_string(), (child_r.raw() as u64).to_string()]);

		let (tx, rx) = Self::channel(child_w, parent_r);
		Ok(Self { command, tx, rx })
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

	/// Spawns the child process and returns it along with a [`Viaduct`](crate::Viaduct).
	pub fn build(mut self) -> Result<(Viaduct<Rpc, Request>, Child), std::io::Error> {
		struct KillHandle(Option<Child>);
		impl Drop for KillHandle {
			#[inline]
			fn drop(&mut self) {
				if let Some(child) = &mut self.0 {
					child.kill().ok();
				}
			}
		}

		self.tx.0.state.lock().tx.write_all(chan::HELLO)?;

		let mut child = KillHandle(Some(self.command.spawn()?));

		let mut hello = [0u8; chan::HELLO.len()];
		self.rx.rx.read_exact(&mut hello)?;
		if hello != chan::HELLO {
			return Err(std::io::Error::new(
				std::io::ErrorKind::BrokenPipe,
				"Child process didn't respond with hello message",
			));
		}

		Ok(((self.tx, self.rx), unsafe { child.0.take().unwrap_unchecked() }))
	}

	fn channel(tx: UnnamedPipeWriter, rx: UnnamedPipeReader) -> Viaduct<Rpc, Request> {
		let tx = ViaductTx(Arc::new(ViaductTxInner {
			condvar: Condvar::new(),
			state: Mutex::new(ViaductTxState {
				buf: Vec::new(),
				tx,
				_phantom: Default::default(),
			})
		}));
		let rx = ViaductRx {
			buf: Vec::new(),
			tx: tx.clone(),
			rx,
		};
		(tx, rx)
	}

	unsafe fn child_handshake(parent_w: u64, child_r: u64) -> Result<Viaduct<Rpc, Request>, ChildError> {
		let parent_w = unsafe { UnnamedPipeWriter::from_raw(parent_w as _) };
		let child_r = unsafe { UnnamedPipeReader::from_raw(child_r as _) };

		let (tx, mut rx) = Self::channel(parent_w, child_r);

		if tx.0.state.lock().tx.write_all(chan::HELLO).is_err() {
			return Err(ChildError);
		}

		let mut hello = [0u8; chan::HELLO.len()];
		if rx.rx.read_exact(&mut hello).is_err() {
			return Err(ChildError);
		}

		if hello != chan::HELLO {
			return Err(ChildError);
		}

		Ok((tx, rx))
	}
}
