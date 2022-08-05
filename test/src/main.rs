use std::process::Command;

#[cfg_attr(feature = "speedy", derive(speedy::Writable, speedy::Readable))]
#[cfg_attr(feature = "bincode", derive(serde::Serialize, serde::Deserialize))]
struct DummyRpc {
	magic: u32,
}
#[cfg_attr(feature = "speedy", derive(speedy::Writable, speedy::Readable))]
#[cfg_attr(feature = "bincode", derive(serde::Serialize, serde::Deserialize))]
struct DummyRequest {
	magic: u32,
}
#[cfg_attr(feature = "speedy", derive(speedy::Writable, speedy::Readable))]
#[cfg_attr(feature = "bincode", derive(serde::Serialize, serde::Deserialize))]
struct DummyResponse {
	magic: u32,
}

fn main() {
	match unsafe { viaduct::ViaductBuilder::child_with_args() } {
		Ok(((tx, rx), mut args)) => {
			std::thread::spawn(move || {
				rx.run(
					|rpc: DummyRpc| {
						assert_eq!(rpc.magic, 69);
						println!("[CHILD] RPC received: {}", rpc.magic);
					},
					|request: DummyRequest| {
						assert_eq!(request.magic, 420);
						println!("[CHILD] Request received: {}", request.magic);
						DummyResponse { magic: 42069 }
					},
				)
				.unwrap();
			});

			tx.rpc(DummyRpc { magic: 69 }).unwrap();

			let response = tx.request(DummyRequest { magic: 420 }).unwrap();
			assert_eq!(response.magic, 42069);
			println!("[CHILD] Response received: {}", response.magic);

			assert_eq!(args.nth(1).as_deref(), Some("Viaduct test!"));
		}

		Err(_) => {
			let ((tx, rx), mut child) = viaduct::ViaductBuilder::parent(Command::new(std::env::current_exe().unwrap()))
				.unwrap()
				.arg("Viaduct test!")
				.build()
				.unwrap();

			std::thread::spawn(move || {
				rx.run(
					|rpc: DummyRpc| {
						assert_eq!(rpc.magic, 69);
						println!("[PARENT] RPC received: {}", rpc.magic);
					},
					|request: DummyRequest| {
						assert_eq!(request.magic, 420);
						println!("[PARENT] Request received: {}", request.magic);
						DummyResponse { magic: 42069 }
					},
				)
				.unwrap();
			});

			tx.rpc(DummyRpc { magic: 69 }).unwrap();
			assert_eq!(tx.request(DummyRequest { magic: 420 }).unwrap().magic, 42069);

			child.wait().unwrap();
		}
	}
}
