use std::sync::{Arc, Barrier};
use std::{io::Write, process::Command};
use viaduct::{ViaductChild, ViaductDeserialize, ViaductEvent, ViaductParent, ViaductSerialize, ViaductTx};

#[derive(Clone, Copy, Debug)]
struct Add {
	a: u32,
	b: u32,
}
impl ViaductSerialize for Add {
	type Error = std::convert::Infallible;

	fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
		buf.write_all(&self.a.to_ne_bytes()).unwrap();
		buf.write_all(&self.b.to_ne_bytes()).unwrap();
		Ok(())
	}
}
impl ViaductDeserialize for Add {
	type Error = std::convert::Infallible;

	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
		Ok(Self {
			a: u32::from_ne_bytes(bytes[0..4].try_into().unwrap()),
			b: u32::from_ne_bytes(bytes[4..8].try_into().unwrap()),
		})
	}
}

const MATH_PROBLEMS: &[(Add, u32)] = &[
	(Add { a: 1, b: 2 }, 3),
	(Add { a: 3, b: 4 }, 7),
	(Add { a: 5, b: 6 }, 11),
	(Add { a: 7, b: 8 }, 15),
	(Add { a: 9, b: 10 }, 19),
];

fn parallel_maths<RpcTx: ViaductSerialize + Send + Sync + 'static, RpcRx: ViaductDeserialize + Send + Sync + 'static>(
	tx: ViaductTx<RpcTx, Add, RpcRx, Add>,
) {
	let mut threads = Vec::with_capacity(MATH_PROBLEMS.len());
	let barrier = Arc::new(Barrier::new(MATH_PROBLEMS.len()));
	for (problem, answer) in MATH_PROBLEMS {
		let tx = tx.clone();
		let barrier = barrier.clone();
		threads.push(std::thread::spawn(move || {
			barrier.wait();
			assert_eq!(tx.request::<u32>(*problem).unwrap().unwrap(), *answer);
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
		// If something is wrong, main will block forever. So kill it after 30 seconds.
		std::thread::sleep(std::time::Duration::from_secs(30));
		std::process::exit(33);
	});

	let named_thread = match unsafe { ViaductChild::<(), Add, (), Add>::new().build_with_args() } {
		// We're the parent process
		Err(_) => std::thread::Builder::new()
			.name("parent".to_string())
			.spawn(|| {
				println!("parent pid {:?}", std::process::id());

				let ((tx, rx), mut child) = ViaductParent::<(), Add, (), Add>::new(Command::new(std::env::current_exe().unwrap()))
					.unwrap()
					.arg("Viaduct test!")
					.build()
					.unwrap();

				let (shutdown_tx, shutdown_rx) = std::sync::mpsc::sync_channel(1);

				std::thread::Builder::new()
					.name("parent event loop".to_string())
					.spawn(move || {
						rx.run(|event| match event {
							ViaductEvent::Rpc(_) => shutdown_tx.try_send(()).unwrap(),
							ViaductEvent::Request { request, responder } => {
								responder.respond(request.a + request.b).unwrap();
							}
						})
						.unwrap();
					})
					.unwrap();

				parallel_maths(tx.clone());
				tx.rpc(()).unwrap();

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
							rx.run(|event| match event {
								ViaductEvent::Rpc(_) => shutdown_tx.try_send(()).unwrap(),
								ViaductEvent::Request { request, responder } => {
									responder.respond(request.a + request.b).unwrap();
								}
							})
							.unwrap();
						})
						.unwrap();

					parallel_maths(tx.clone());
					tx.rpc(()).unwrap();

					shutdown_rx.recv().unwrap();
				})
				.unwrap()
		}
	};

	named_thread.join().ok();
}
