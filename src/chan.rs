use crate::serde::Pipeable;
use interprocess::unnamed_pipe::{UnnamedPipeReader, UnnamedPipeWriter};
use parking_lot::{Condvar, Mutex};
use std::{
	io::{Read, Write},
	marker::PhantomData,
	mem::size_of,
	sync::Arc,
};
use uuid::Uuid;

pub(super) const HELLO: &[u8] = b"Read this if you are a beautiful strong unnamed pipe who don't need no handles";

/// A channel pair for sending and receiving data across the viaduct.
pub type Viaduct<RpcTx, RequestTx, RpcRx, RequestRx> = (
	ViaductTx<RpcTx, RequestTx, RpcRx, RequestRx>,
	ViaductRx<RpcTx, RequestTx, RpcRx, RequestRx>,
);

// The following two structs allow us to avoid dynamic dispatch for serializing responses.
// ViaductResponded forces the user to return a response in their request handler.
// ViaductResponse allows the user to send anything Pipeable as a response.
#[doc(hidden)]
pub struct ViaductResponded(PhantomData<()>);
/// Use [`ViaductResponse::respond`] to send a response to the other side.
pub struct ViaductResponse<'a>(&'a mut Vec<u8>);
impl ViaductResponse<'_> {
	/// Sends a response to the other side.
	///
	/// You can send whatever type you want, as long as it implements [`Pipeable`].
	///
	/// # Panics
	///
	/// This function won't panic, but the peer process will panic if you send a different type to what it was expecting.
	///
	/// # Example
	///
	/// ```no_run
	/// # use viaduct::doctest::*;
	/// # let rx = unsafe { viaduct::ViaductBuilder::<ExampleRpc, ExampleRequest, ExampleRpc, ExampleRequest>::child() }.unwrap().1;
	/// rx.run(
	///     |rpc: ExampleRpc| match rpc {
	///         ExampleRpc::Cow => println!("Moo"),
	///         ExampleRpc::Pig => println!("Oink"),
	///         ExampleRpc::Horse => println!("Neigh"),
	///     },
	///
	///     |request: ExampleRequest, tx| match request {
	///         ExampleRequest::DoAFrontflip => {
	///             println!("Doing a frontflip!");
	///             tx.respond(Ok::<_, FrontflipError>(()))
	///         },
	///
	///         ExampleRequest::DoABackflip => {
	///             println!("Doing a backflip!");
	///             tx.respond(Ok::<_, BackflipError>(()))
	///         },
	///     },
	/// ).unwrap();
	/// ```
	#[must_use = "You must return this from your request handler"]
	pub fn respond(self, response: impl Pipeable) -> ViaductResponded {
		response
			.to_pipeable({
				self.0.clear();
				self.0
			})
			.expect("Failed to serialize response");

		ViaductResponded(Default::default())
	}
}

/// The receiving side of a viaduct.
pub struct ViaductRx<RpcTx, RequestTx, RpcRx, RequestRx> {
	pub(super) buf: Vec<u8>,
	pub(super) tx: ViaductTx<RpcTx, RequestTx, RpcRx, RequestRx>,
	pub(super) rx: UnnamedPipeReader,
	pub(super) _phantom: PhantomData<RequestRx>,
}
impl<RpcTx, RequestTx, RpcRx, RequestRx> ViaductRx<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: Pipeable,
	RpcRx: Pipeable,
	RequestTx: Pipeable,
	RequestRx: Pipeable,
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
	/// # use viaduct::doctest::*;
	/// # let rx = unsafe { viaduct::ViaductBuilder::<ExampleRpc, ExampleRequest, ExampleRpc, ExampleRequest>::child() }.unwrap().1;
	/// std::thread::spawn(move || {
	///     rx.run(
	///         |rpc: ExampleRpc| match rpc {
	///             ExampleRpc::Cow => println!("Moo"),
	///             ExampleRpc::Pig => println!("Oink"),
	///             ExampleRpc::Horse => println!("Neigh"),
	///         },
	///
	///         |request: ExampleRequest, tx| match request {
	///             ExampleRequest::DoAFrontflip => {
	///                 println!("Doing a frontflip!");
	///                 tx.respond(Ok::<_, FrontflipError>(()))
	///             },
	///
	///             ExampleRequest::DoABackflip => {
	///                 println!("Doing a backflip!");
	///                 tx.respond(Ok::<_, BackflipError>(()))
	///             },
	///         },
	///     ).unwrap();
	/// });
	/// ```
	pub fn run<RpcHandler, RequestHandler>(mut self, mut rpc_handler: RpcHandler, mut request_handler: RequestHandler) -> Result<(), std::io::Error>
	where
		RpcHandler: FnMut(RpcRx),
		RequestHandler: for<'a> FnMut(RequestRx, ViaductResponse<'a>) -> ViaductResponded,
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
				0 => {
					recv_into_buf(&mut self.rx, &mut self.buf)?;

					let rpc = RpcRx::from_pipeable(&self.buf).expect("Failed to deserialize RpcRx");
					rpc_handler(rpc);
				}

				1 => {
					let request_id = {
						let mut request_id = [0u8; 16];
						self.rx.read_exact(&mut request_id)?;
						request_id
					};

					recv_into_buf(&mut self.rx, &mut self.buf)?;

					let request = RequestRx::from_pipeable(&self.buf).expect("Failed to deserialize RequestRx");

					self.buf.clear();
					request_handler(request, ViaductResponse(&mut self.buf));

					let mut tx = self.tx.0.state.lock();
					let ViaductTxState { tx, .. } = &mut *tx;
					tx.write_all(&[2])?;
					tx.write_all(&request_id)?;
					tx.write_all(&u64::to_ne_bytes(self.buf.len() as _))?;
					tx.write_all(&self.buf)?;
				}

				2 => {
					let mut response = self.tx.0.response.lock();
					self.tx
						.0
						.response_condvar
						.wait_while(&mut response, |response| response.for_request_id.is_some());

					response.for_request_id = Some({
						let mut request_id = [0u8; 16];
						self.rx.read_exact(&mut request_id)?;
						Uuid::from_bytes(request_id)
					});

					// Receive the response into the sender's buffer
					response.buf.clear();
					recv_into_buf(&mut self.rx, &mut response.buf)?;

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
	for_request_id: Option<Uuid>,
	buf: Vec<u8>,
}

/// The sending side of a viaduct.
///
/// This handle can be freely cloned and sent across threads.
pub struct ViaductTx<RpcTx, RequestTx, RpcRx, RequestRx>(pub(super) Arc<ViaductTxInner<RpcTx, RequestTx, RpcRx, RequestRx>>);

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
	RpcTx: Pipeable,
	RpcRx: Pipeable,
	RequestTx: Pipeable,
	RequestRx: Pipeable,
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
	RpcTx: Pipeable,
	RpcRx: Pipeable,
	RequestTx: Pipeable,
	RequestRx: Pipeable,
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
	/// Only one request can be made at a time by any thread. A single request will block all threads trying to send requests and RPCs.
	///
	/// # Panics
	///
	/// This function will panic if the peer process doesn't send the expected type (`Response`) as the response.
	pub fn request<Response: Pipeable>(&self, request: RequestTx) -> Result<Response, std::io::Error> {
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
			.wait_while(&mut response, |response| response.for_request_id != Some(request_id));

		// Take the request ID out of state
		debug_assert_eq!(response.for_request_id, Some(request_id));
		response.for_request_id = None;

		// Notify the condvar because the writer half might be waiting for the request ID to become None
		self.0.response_condvar.notify_all();

		// Deserialize the response and return it
		Ok(Response::from_pipeable(&response.buf).expect("Failed to deserialize Response"))
	}
}
impl<RpcTx, RequestTx, RpcRx, RequestRx> Clone for ViaductTx<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: Pipeable,
	RpcRx: Pipeable,
	RequestTx: Pipeable,
	RequestRx: Pipeable,
{
	#[inline]
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}
