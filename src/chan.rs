use crate::serde::Pipeable;
use interprocess::unnamed_pipe::{UnnamedPipeReader, UnnamedPipeWriter};
use parking_lot::Mutex;
use std::{
	io::{Read, Write},
	marker::PhantomData,
	mem::size_of,
	sync::Arc,
};

pub(super) const HELLO: &[u8] = b"Read this if you are a beautiful strong unnamed pipe who don't need no handles";

/// A channel pair for sending and receiving data across the viaduct.
pub type Viaduct<Rpc, Request, Response> = (ViaductTx<Rpc, Request, Response>, ViaductRx<Rpc, Request, Response>);

/// The receiving side of a viaduct.
pub struct ViaductRx<Rpc, Request, Response> {
	pub(super) buf: Vec<u8>,
	pub(super) request_rx: crossbeam_channel::Receiver<oneshot::Sender<Response>>,
	pub(super) tx: ViaductTx<Rpc, Request, Response>,
	pub(super) rx: UnnamedPipeReader,
}
impl<Rpc, Request, Response> ViaductRx<Rpc, Request, Response>
where
	Rpc: Pipeable,
	Request: Pipeable,
	Response: Pipeable,
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
	///         |request: ExampleRequest| match request {
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
		RequestHandler: FnMut(Request) -> Response,
	{
		loop {
			let len = {
				let mut len = [0u8; size_of::<u64>()];
				self.rx.read_exact(&mut len)?;
				usize::try_from(u64::from_ne_bytes(len)).expect("Viaduct packet was larger than what this architecture can handle")
			};
			self.buf.resize(len, 0);
			self.rx.read_exact(&mut self.buf)?;

			let packet_type = {
				let mut packet_type = [0u8];
				self.rx.read_exact(&mut packet_type)?;
				packet_type[0]
			};
			match packet_type {
				0 => {
					let rpc = Rpc::from_pipeable(&self.buf);
					rpc_handler(rpc);
				}

				1 => {
					let request = Request::from_pipeable(&self.buf);

					request_handler(request).to_pipeable({
						self.buf.clear();
						&mut self.buf
					});

					let mut tx = self.tx.0.lock();
					tx.tx.write_all(&u64::to_ne_bytes(self.buf.len() as _))?;
					tx.tx.write_all(&self.buf)?;
					tx.tx.write_all(&[2])?;
				}

				2 => {
					let response = Response::from_pipeable(&self.buf);
					if self.request_rx.recv().ok().and_then(|tx| tx.send(response).ok()).is_none() {
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
pub struct ViaductTx<Rpc, Request, Response>(pub(super) Arc<Mutex<ViaductTxInner<Rpc, Request, Response>>>);
pub(super) struct ViaductTxInner<Rpc, Request, Response> {
	pub(super) buf: Vec<u8>,
	pub(super) request_tx: crossbeam_channel::Sender<oneshot::Sender<Response>>,
	pub(super) tx: UnnamedPipeWriter,
	pub(super) _phantom: PhantomData<(Rpc, Request)>,
}
impl<Rpc, Request, Response> ViaductTx<Rpc, Request, Response>
where
	Rpc: Pipeable,
	Request: Pipeable,
	Response: Pipeable,
{
	/// Sends an RPC to the peer process.
	pub fn rpc(&self, rpc: Rpc) -> Result<(), std::io::Error> {
		let mut state = self.0.lock();

		let ViaductTxInner { buf, tx, .. } = &mut *state;

		rpc.to_pipeable({
			buf.clear();
			buf
		});

		tx.write_all(&u64::to_ne_bytes(buf.len() as _))?;
		tx.write_all(&*buf)?;
		tx.write_all(&[0])?;

		Ok(())
	}

	/// Sends a request to the peer process and awaits a response.
	///
	/// Only one request can be made at a time by any thread. A single request will block all threads trying to send requests and RPCs.
	pub fn request(&self, request: Request) -> Result<Response, std::io::Error> {
		let response_rx = {
			let mut state = self.0.lock();

			let (response_tx, response_rx) = oneshot::channel::<Response>();

			if state.request_tx.send(response_tx).is_err() {
				return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "ViaductRx hung up"));
			}

			let ViaductTxInner { buf, tx, .. } = &mut *state;

			request.to_pipeable({
				buf.clear();
				buf
			});

			tx.write_all(&u64::to_ne_bytes(buf.len() as _))?;
			tx.write_all(&*buf)?;
			tx.write_all(&[1])?;

			response_rx
		};

		response_rx
			.recv()
			.map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "ViaductRx hung up"))
	}
}
impl<Rpc, Request, Response> Clone for ViaductTx<Rpc, Request, Response>
where
	Rpc: Pipeable,
	Request: Pipeable,
	Response: Pipeable,
{
	#[inline]
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}
