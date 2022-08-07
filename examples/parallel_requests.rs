use std::sync::{Arc, Barrier};
use std::{io::Write, process::Command};
use viaduct::{Pipeable, ViaductTx};

struct Shutdown;
impl Pipeable for Shutdown {
	type DeserializeError = std::convert::Infallible;
	type SerializeError = std::convert::Infallible;

	fn from_pipeable(_bytes: &[u8]) -> Result<Self, Self::DeserializeError> {
		Ok(Self)
	}

	fn to_pipeable(&self, _buf: &mut Vec<u8>) -> Result<(), Self::SerializeError> {
		Ok(())
	}
}

#[derive(Clone, Copy, Debug)]
struct Add {
	a: u32,
	b: u32,
}
impl Pipeable for Add {
	type DeserializeError = std::convert::Infallible;
	type SerializeError = std::convert::Infallible;

	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::DeserializeError> {
		Ok(Self {
			a: u32::from_ne_bytes(bytes[0..4].try_into().unwrap()),
			b: u32::from_ne_bytes(bytes[4..8].try_into().unwrap()),
		})
	}

	fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::SerializeError> {
		buf.write_all(&self.a.to_ne_bytes()).unwrap();
		buf.write_all(&self.b.to_ne_bytes()).unwrap();
		Ok(())
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AddResult {
	result: u32,
}
impl Pipeable for AddResult {
	type DeserializeError = std::convert::Infallible;
	type SerializeError = std::convert::Infallible;

	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::DeserializeError> {
		Ok(Self {
			result: u32::from_ne_bytes(bytes[0..4].try_into().unwrap()),
		})
	}

	fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::SerializeError> {
		buf.write_all(&self.result.to_ne_bytes()).unwrap();
		Ok(())
	}
}

const MATH_PROBLEMS: &[(Add, AddResult)] = &[
	(Add { a: 1, b: 2 }, AddResult { result: 3 }),
	(Add { a: 3, b: 4 }, AddResult { result: 7 }),
	(Add { a: 5, b: 6 }, AddResult { result: 11 }),
	(Add { a: 7, b: 8 }, AddResult { result: 15 }),
	(Add { a: 9, b: 10 }, AddResult { result: 19 }),
];

fn parallel_maths<RpcTx: Pipeable + Send + Sync + 'static, RpcRx: Pipeable + Send + Sync + 'static>(tx: ViaductTx<RpcTx, Add, RpcRx, Add>) {
	let mut threads = Vec::with_capacity(MATH_PROBLEMS.len());
	let barrier = Arc::new(Barrier::new(MATH_PROBLEMS.len()));
	for (problem, answer) in MATH_PROBLEMS {
		let tx = tx.clone();
		let barrier = barrier.clone();
		threads.push(std::thread::spawn(move || {
			barrier.wait();
			assert_eq!(tx.request::<AddResult>(*problem).unwrap(), *answer);
			println!("[{}] {problem:?} = {answer:?}", std::process::id());
		}));
	}
	threads.into_iter().for_each(|thread| {
		thread.join().ok();
	});
	println!("[{}] Maths worked!", std::process::id());
}

fn main() {
	std::thread::spawn(|| {
		// If something is wrong, main will block forever. So kill it after 10 seconds.
		std::thread::sleep(std::time::Duration::from_secs(10));
		std::process::exit(1);
	});

	let named_thread = match unsafe { viaduct::ViaductBuilder::<Shutdown, Add, Shutdown, Add>::child_with_args() } {
		// We're the parent process
		Err(_) => std::thread::Builder::new()
			.name("parent".to_string())
			.spawn(|| {
				println!("parent pid {:?}", std::process::id());

				let ((tx, rx), mut child) =
					viaduct::ViaductBuilder::<Shutdown, Add, Shutdown, Add>::parent(Command::new(std::env::current_exe().unwrap()))
						.unwrap()
						.arg("Viaduct test!")
						.build()
						.unwrap();

				let (shutdown_tx, shutdown_rx) = std::sync::mpsc::sync_channel(1);

				std::thread::Builder::new()
					.name("parent event loop".to_string())
					.spawn(move || {
						rx.run(
							|_| {
								shutdown_tx.try_send(()).unwrap();
							},
							|request, tx| {
								tx.respond(AddResult {
									result: request.a + request.b,
								})
							},
						)
						.unwrap();
					})
					.unwrap();

				parallel_maths(tx.clone());
				tx.rpc(Shutdown).unwrap();

				shutdown_rx.recv().unwrap();

				child.wait().unwrap();
			})
			.unwrap(),

		// We're the child process
		Ok(((tx, rx), mut args)) => {
			assert_eq!(args.nth(1).as_deref(), Some("Viaduct test!"));

			std::thread::Builder::new()
				.name("child".to_string())
				.spawn(move || {
					println!("child pid {:?}", std::process::id());

					let (shutdown_tx, shutdown_rx) = std::sync::mpsc::sync_channel(1);

					std::thread::Builder::new()
						.name("child event loop".to_string())
						.spawn(move || {
							rx.run(
								|_| {
									shutdown_tx.try_send(()).unwrap();
								},
								|request, tx| {
									tx.respond(AddResult {
										result: request.a + request.b,
									})
								},
							)
							.unwrap();
						})
						.unwrap();

					parallel_maths(tx.clone());
					tx.rpc(Shutdown).unwrap();

					shutdown_rx.recv().unwrap();
				})
				.unwrap()
		}
	};

	named_thread.join().ok();
}
