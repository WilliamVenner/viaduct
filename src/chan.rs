use crate::serde::Pipeable;
use interprocess::unnamed_pipe::{UnnamedPipeReader, UnnamedPipeWriter};
use parking_lot::{Mutex, Condvar};
use std::{
	io::{Read, Write},
	marker::PhantomData,
	mem::size_of,
	sync::Arc,
};

pub(super) const HELLO: &[u8] = b"Read this if you are a beautiful strong unnamed pipe who don't need no handles";

/// A channel pair for sending and receiving data across the viaduct.
pub type Viaduct<Rpc, Request> = (ViaductTx<Rpc, Request>, ViaductRx<Rpc, Request>);

// The following two structs allow us to avoid dynamic dispatch for serializing responses.
// ViaductResponded forces the user to return a response in their request handler.
// ViaductResponse allows the user to send anything Pipeable as a response.
#[doc(hidden)]
pub struct ViaductResponded(PhantomData<()>);
/// Use [`ViaductResponse::respond`] to send a response to the other side.
pub struct ViaductResponse<'a>(&'a mut Vec<u8>);
impl ViaductResponse<'_> {
	/// Sends a response to the other side.
	#[must_use = "You must return this from your request handler"]
	pub fn respond(self, response: impl Pipeable) -> ViaductResponded {
		response.to_pipeable({
			self.0.clear();
			self.0
		});
		ViaductResponded(Default::default())
	}
}

/// The receiving side of a viaduct.
pub struct ViaductRx<Rpc, Request> {
	pub(super) buf: Vec<u8>,
	pub(super) tx: ViaductTx<Rpc, Request>,
	pub(super) rx: UnnamedPipeReader,
}
impl<Rpc, Request> ViaductRx<Rpc, Request>
where
	Rpc: Pipeable,
	Request: Pipeable,
{
	/// Runs the event loop. This function will never return unless an error occurs.
	///
	/// # Example
	///
	/// ```ignore
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
	///                 ExampleResponse::FrontflipOk
	///             },
	///
	///             ExampleRequest::DoABackflip => {
	///                 println!("Doing a backflip!");
	///                 ExampleResponse::BackflipOk
	///             },
	///         },
	///     ).unwrap();
	/// });
	/// ```
	pub fn run<RpcHandler, RequestHandler>(mut self, mut rpc_handler: RpcHandler, mut request_handler: RequestHandler) -> Result<(), std::io::Error>
	where
		RpcHandler: FnMut(Rpc),
		RequestHandler: for<'a> FnMut(Request, ViaductResponse<'a>) -> ViaductResponded,
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

					let rpc = Rpc::from_pipeable(&self.buf);
					rpc_handler(rpc);
				}

				1 => {
					recv_into_buf(&mut self.rx, &mut self.buf)?;

					let request = Request::from_pipeable(&self.buf);

					self.buf.clear();
					request_handler(request, ViaductResponse(&mut self.buf));

					let mut tx = self.tx.0.state.lock();
					tx.tx.write_all(&[2])?;
					tx.tx.write_all(&u64::to_ne_bytes(self.buf.len() as _))?;
					tx.tx.write_all(&self.buf)?;
				}

				2 => {
					// Lock the sender side
					let mut lock = self.tx.0.state.lock();

					// Receive the response into the sender's buffer
					lock.buf.clear();
					recv_into_buf(&mut self.rx, &mut lock.buf)?;

					// Tell the sender that the response is ready and in their buffer!
					if !self.tx.0.condvar.notify_one() {
						return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Viaduct request channel was closed"));
					}
				}

				_ => unreachable!(),
			}
		}
	}
}

/// The sending side of a viaduct.
///
/// This handle can be freely cloned and sent across threads.
pub struct ViaductTx<Rpc, Request>(pub(super) Arc<ViaductTxInner<Rpc, Request>>);
pub(super) struct ViaductTxInner<Rpc, Request> {
	pub(super) state: Mutex<ViaductTxState<Rpc, Request>>,

	/// Condvar used to synchronize passing a response to the other side of the viaduct.
	pub(super) condvar: Condvar
}
pub(super) struct ViaductTxState<Rpc, Request> {
	pub(super) buf: Vec<u8>,
	pub(super) tx: UnnamedPipeWriter,
	pub(super) _phantom: PhantomData<(Rpc, Request)>,
}
impl<Rpc, Request> ViaductTx<Rpc, Request>
where
	Rpc: Pipeable,
	Request: Pipeable,
{
	/// Sends an RPC to the peer process.
	pub fn rpc(&self, rpc: Rpc) -> Result<(), std::io::Error> {
		let mut state = self.0.state.lock();

		let ViaductTxState { buf, tx, .. } = &mut *state;

		rpc.to_pipeable({
			buf.clear();
			buf
		});

		tx.write_all(&[0])?;
		tx.write_all(&u64::to_ne_bytes(buf.len() as _))?;
		tx.write_all(&*buf)?;

		Ok(())
	}

	/// Sends a request to the peer process and awaits a response.
	///
	/// Only one request can be made at a time by any thread. A single request will block all threads trying to send requests and RPCs.
	pub fn request<Response: Pipeable>(&self, request: Request) -> Result<Response, std::io::Error> {
		let mut state = self.0.state.lock();

		// Send the request down the wire
		{
			let ViaductTxState { buf, tx, .. } = &mut *state;

			request.to_pipeable({
				buf.clear();
				buf
			});

			tx.write_all(&[1])?;
			tx.write_all(&u64::to_ne_bytes(buf.len() as _))?;
			tx.write_all(&*buf)?;
		}

		// Unlock the mutex and wait for the receiving side to send the response
		self.0.condvar.wait(&mut state);

		// Deserialize the response and return it
		Ok(Response::from_pipeable(&state.buf))
	}
}
impl<Rpc, Request> Clone for ViaductTx<Rpc, Request>
where
	Rpc: Pipeable,
	Request: Pipeable,
{
	#[inline]
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}
