use crate::{
	serde::{ViaductDeserialize, ViaductSerialize},
	ViaductEvent,
};
use interprocess::unnamed_pipe::{UnnamedPipeReader, UnnamedPipeWriter};
use parking_lot::{Condvar, Mutex};
use std::{
	io::{Read, Write},
	marker::PhantomData,
	mem::size_of,
	sync::Arc,
	time::{Duration, Instant},
};
use uuid::Uuid;

const RPC: u8 = 0;
const REQUEST: u8 = 1;
const SOME_RESPONSE: u8 = 2;
const NONE_RESPONSE: u8 = 3;

pub(super) const HELLO: &[u8] = b"Read this if you are a beautiful strong unnamed pipe who don't need no handles";

/// A channel pair for sending and receiving data across the viaduct.
pub type Viaduct<RpcTx, RequestTx, RpcRx, RequestRx> = (
	ViaductTx<RpcTx, RequestTx, RpcRx, RequestRx>,
	ViaductRx<RpcTx, RequestTx, RpcRx, RequestRx>,
);
/// Use [`ViaductRequestResponder::respond`] to send a response to the other side.
pub struct ViaductRequestResponder<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize,
{
	tx: ViaductTx<RpcTx, RequestTx, RpcRx, RequestRx>,
	request_id: Uuid,
}
impl<RpcTx, RequestTx, RpcRx, RequestRx> ViaductRequestResponder<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize,
{
	/// Sends a response to the other side.
	///
	/// You can send whatever type you want, as long as it implements [`ViaductSerialize`].
	///
	/// # Panics
	///
	/// This function won't panic, but the peer process will panic if you send a different type to what it was expecting.
	///
	/// # Example
	///
	/// ```no_run
	/// # use viaduct::{ViaductEvent, ViaductChild, doctest::*};
	/// # let rx = unsafe { ViaductChild::<ExampleRpc, ExampleRequest, ExampleRpc, ExampleRequest>::new().build() }.unwrap().1;
	/// rx.run(|event| match event {
	///     ViaductEvent::Rpc(rpc) => match rpc {
	///         ExampleRpc::Cow => println!("Moo"),
	///         ExampleRpc::Pig => println!("Oink"),
	///         ExampleRpc::Horse => println!("Neigh"),
	///     },
	///
	///     ViaductEvent::Request { request, responder } => match request {
	///         ExampleRequest::DoAFrontflip => {
	///             println!("Doing a frontflip!");
	///             responder.respond(Ok::<_, FrontflipError>(())).unwrap();
	///         },
	///
	///         ExampleRequest::DoABackflip => {
	///             println!("Doing a backflip!");
	///             responder.respond(Ok::<_, BackflipError>(())).unwrap();
	///         },
	///     }
	/// }).unwrap();
	/// ```
	pub fn respond(self, response: impl ViaductSerialize) -> Result<(), std::io::Error> {
		let mut state = self.tx.0.state.lock();
		let ViaductTxState { tx, buf, .. } = &mut *state;

		response
			.to_pipeable({
				buf.clear();
				buf
			})
			.expect("Failed to serialize response");

		tx.write_all(&[2])?;
		tx.write_all(&*self.request_id.as_bytes())?;
		tx.write_all(&u64::to_ne_bytes(buf.len() as _))?;
		tx.write_all(buf)?;

		Ok(())
	}
}
impl<RpcTx, RequestTx, RpcRx, RequestRx> Drop for ViaductRequestResponder<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize,
{
	fn drop(&mut self) {
		let mut state = self.tx.0.state.lock();
		let ViaductTxState { tx, .. } = &mut *state;

		(|| {
			tx.write_all(&[3])?;
			tx.write_all(&*self.request_id.as_bytes())?;
			Ok::<_, std::io::Error>(())
		})()
		.unwrap();
	}
}

/// The receiving side of a viaduct.
pub struct ViaductRx<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize,
{
	pub(super) buf: Vec<u8>,
	pub(super) tx: ViaductTx<RpcTx, RequestTx, RpcRx, RequestRx>,
	pub(super) rx: UnnamedPipeReader,
	pub(super) _phantom: PhantomData<RequestRx>,
}
impl<RpcTx, RequestTx, RpcRx, RequestRx> ViaductRx<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestTx: ViaductSerialize,
	RequestRx: ViaductDeserialize,
{
	/// Runs the event loop. This function will never return unless an error occurs.
	///
	/// # Panics
	///
	/// This function will panic if the peer process sends some data (RPC or request) and this process fails to deserialize it.
	///
	/// # Example
	///
	/// ```no_run
	/// # use viaduct::{ViaductEvent, ViaductChild, doctest::*};
	/// # let rx = unsafe { ViaductChild::<ExampleRpc, ExampleRequest, ExampleRpc, ExampleRequest>::new().build() }.unwrap().1;
	/// rx.run(|event| match event {
	///     ViaductEvent::Rpc(rpc) => match rpc {
	///         ExampleRpc::Cow => println!("Moo"),
	///         ExampleRpc::Pig => println!("Oink"),
	///         ExampleRpc::Horse => println!("Neigh"),
	///     },
	///
	///     ViaductEvent::Request { request, responder } => match request {
	///         ExampleRequest::DoAFrontflip => {
	///             println!("Doing a frontflip!");
	///             responder.respond(Ok::<_, FrontflipError>(())).unwrap();
	///         },
	///
	///         ExampleRequest::DoABackflip => {
	///             println!("Doing a backflip!");
	///             responder.respond(Ok::<_, BackflipError>(())).unwrap();
	///         },
	///     }
	/// }).unwrap();
	/// ```
	pub fn run<EventHandler>(mut self, mut event_handler: EventHandler) -> Result<(), std::io::Error>
	where
		EventHandler: FnMut(ViaductEvent<RpcTx, RequestTx, RpcRx, RequestRx>),
	{
		let recv_into_buf = |rx: &mut UnnamedPipeReader, buf: &mut Vec<u8>| -> Result<(), std::io::Error> {
			let len = {
				let mut len = [0u8; size_of::<u64>()];
				rx.read_exact(&mut len)?;
				usize::try_from(u64::from_ne_bytes(len)).expect("Viaduct packet was larger than what this architecture can handle")
			};
			buf.resize(len, 0);
			rx.read_exact(buf)?;
			Ok(())
		};

		loop {
			let packet_type = {
				let mut packet_type = [0u8];
				self.rx.read_exact(&mut packet_type)?;
				packet_type[0]
			};
			match packet_type {
				RPC => {
					recv_into_buf(&mut self.rx, &mut self.buf)?;

					let rpc = RpcRx::from_pipeable(&self.buf).expect("Failed to deserialize RpcRx");
					event_handler(ViaductEvent::Rpc(rpc));
				}

				REQUEST => {
					let request_id = {
						let mut request_id = [0u8; 16];
						self.rx.read_exact(&mut request_id)?;
						Uuid::from_bytes(request_id)
					};

					recv_into_buf(&mut self.rx, &mut self.buf)?;

					event_handler(ViaductEvent::Request {
						request: RequestRx::from_pipeable(&self.buf).expect("Failed to deserialize RequestRx"),
						responder: ViaductRequestResponder {
							tx: self.tx.clone(),
							request_id,
						},
					});
				}

				SOME_RESPONSE => {
					let mut response = self.tx.0.response.lock();

					response.for_request_id = Some({
						let mut request_id = [0u8; 16];
						self.rx.read_exact(&mut request_id)?;
						(Uuid::from_bytes(request_id), true)
					});

					// Receive the response into the sender's buffer
					response.buf.clear();
					recv_into_buf(&mut self.rx, &mut response.buf)?;

					// Tell the sender that the response is ready and in their buffer!
					self.tx.0.response_condvar.notify_all();
				}

				NONE_RESPONSE => {
					let mut response = self.tx.0.response.lock();

					response.for_request_id = Some({
						let mut request_id = [0u8; 16];
						self.rx.read_exact(&mut request_id)?;
						(Uuid::from_bytes(request_id), false)
					});

					// Tell the sender that the response is ready and in their buffer!
					self.tx.0.response_condvar.notify_all();
				}

				_ => unreachable!(),
			}
		}
	}
}

#[derive(Default)]
pub(super) struct ViaductResponseState {
	for_request_id: Option<(Uuid, bool)>,
	buf: Vec<u8>,
}
impl ViaductResponseState {
	#[inline]
	fn request_id(&self) -> Option<&Uuid> {
		self.for_request_id.as_ref().map(|(id, _)| id)
	}
}

/// The sending side of a viaduct.
///
/// This handle can be freely cloned and sent across threads.
pub struct ViaductTx<RpcTx, RequestTx, RpcRx, RequestRx>(pub(super) Arc<ViaductTxInner<RpcTx, RequestTx, RpcRx, RequestRx>>)
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize;

pub(super) struct ViaductTxInner<RpcTx, RequestTx, RpcRx, RequestRx> {
	pub(super) state: Mutex<ViaductTxState<RpcTx, RequestTx, RpcRx, RequestRx>>,
	pub(super) response: Mutex<ViaductResponseState>,
	pub(super) response_condvar: Condvar,
}

pub(super) struct ViaductTxState<RpcTx, RequestTx, RpcRx, RequestRx> {
	pub(super) tx: UnnamedPipeWriter,
	buf: Vec<u8>,
	_phantom: PhantomData<(RpcTx, RequestTx, RpcRx, RequestRx)>,
}
impl<RpcTx, RequestTx, RpcRx, RequestRx> ViaductTxState<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestTx: ViaductSerialize,
	RequestRx: ViaductDeserialize,
{
	#[inline]
	pub(super) fn new(tx: UnnamedPipeWriter) -> Self {
		Self {
			buf: Vec::new(),
			tx,
			_phantom: Default::default(),
		}
	}
}

impl<RpcTx, RequestTx, RpcRx, RequestRx> ViaductTx<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestTx: ViaductSerialize,
	RequestRx: ViaductDeserialize,
{
	/// Sends an RPC to the peer process.
	///
	/// # Panics
	///
	/// This function won't panic, but the peer process will panic if the RPC is unable to be deserialized.
	pub fn rpc(&self, rpc: RpcTx) -> Result<(), std::io::Error> {
		let mut state = self.0.state.lock();

		let ViaductTxState { buf, tx, .. } = &mut *state;

		rpc.to_pipeable({
			buf.clear();
			buf
		})
		.expect("Failed to serialize RpcTx");

		tx.write_all(&[0])?;
		tx.write_all(&u64::to_ne_bytes(buf.len() as _))?;
		tx.write_all(&*buf)?;

		Ok(())
	}

	/// Sends a request to the peer process and awaits a response.
	///
	/// This will block the current thread.
	///
	/// # Panics
	///
	/// This function will panic if the peer process doesn't send the expected type (`Response`) as the response.
	pub fn request<Response: ViaductDeserialize>(&self, request: RequestTx) -> Result<Option<Response>, std::io::Error> {
		let mut response = self.0.response.lock();

		// Get a request ID
		let request_id = Uuid::new_v4();

		// Send the request down the wire
		{
			let mut state = self.0.state.lock();
			let ViaductTxState { buf, tx, .. } = &mut *state;

			request
				.to_pipeable({
					buf.clear();
					buf
				})
				.expect("Failed to serialize RequestTx");

			tx.write_all(&[1])?;
			tx.write_all(request_id.as_bytes())?;
			tx.write_all(&u64::to_ne_bytes(buf.len() as _))?;
			tx.write_all(&*buf)?;
		}

		self.0
			.response_condvar
			.wait_while(&mut response, |response| response.request_id() != Some(&request_id));

		let (for_request_id, some) = response.for_request_id.take().unwrap();
		debug_assert_eq!(for_request_id, request_id);

		// Notify the condvar because the writer half might be waiting for the request ID to become None
		self.0.response_condvar.notify_all();

		// Deserialize the response and return it
		Ok(if some {
			Some(Response::from_pipeable(&response.buf).expect("Failed to deserialize Response"))
		} else {
			None
		})
	}

	/// Sends a request to the peer process and awaits a response, timing out after an [`Instant`](std::time::Instant) has passed.
	///
	/// This will block the current thread.
	///
	/// # Panics
	///
	/// This function will panic if the peer process doesn't send the expected type (`Response`) as the response.
	pub fn request_timeout_at<Response: ViaductDeserialize>(
		&self,
		timeout_at: Instant,
		request: RequestTx,
	) -> Result<Option<Response>, std::io::Error> {
		let mut response = self
			.0
			.response
			.try_lock_until(timeout_at)
			.ok_or_else(|| std::io::Error::from(std::io::ErrorKind::TimedOut))?;

		// Get a request ID
		let request_id = Uuid::new_v4();

		// Send the request down the wire
		{
			let mut state = self
				.0
				.state
				.try_lock_until(timeout_at)
				.ok_or_else(|| std::io::Error::from(std::io::ErrorKind::TimedOut))?;
			let ViaductTxState { buf, tx, .. } = &mut *state;

			request
				.to_pipeable({
					buf.clear();
					buf
				})
				.expect("Failed to serialize RequestTx");

			tx.write_all(&[1])?;
			tx.write_all(request_id.as_bytes())?;
			tx.write_all(&u64::to_ne_bytes(buf.len() as _))?;
			tx.write_all(&*buf)?;
		}

		if self
			.0
			.response_condvar
			.wait_while_until(&mut response, |response| response.request_id() != Some(&request_id), timeout_at)
			.timed_out()
		{
			return Err(std::io::Error::from(std::io::ErrorKind::TimedOut));
		}

		let (for_request_id, some) = response.for_request_id.take().unwrap();
		debug_assert_eq!(for_request_id, request_id);

		// Notify the condvar because the writer half might be waiting for the request ID to become None
		self.0.response_condvar.notify_all();

		// Deserialize the response and return it
		Ok(if some {
			Some(Response::from_pipeable(&response.buf).expect("Failed to deserialize Response"))
		} else {
			None
		})
	}

	/// Sends a request to the peer process and awaits a response, timing out after the given duration.
	///
	/// This will block the current thread.
	///
	/// # Panics
	///
	/// This function will panic if the peer process doesn't send the expected type (`Response`) as the response.
	#[inline]
	pub fn request_timeout<Response: ViaductDeserialize>(&self, timeout: Duration, request: RequestTx) -> Result<Option<Response>, std::io::Error> {
		self.request_timeout_at(Instant::now() + timeout, request)
	}
}
impl<RpcTx, RequestTx, RpcRx, RequestRx> Clone for ViaductTx<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestTx: ViaductSerialize,
	RequestRx: ViaductDeserialize,
{
	#[inline]
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}
