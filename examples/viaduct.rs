use std::process::Command;

fn main() {
	std::thread::spawn(|| {
		// If something is wrong, main will block forever. So kill it after 10 seconds.
		std::thread::sleep(std::time::Duration::from_secs(10));
		std::process::exit(1);
	});

	let named_thread = match unsafe {
		viaduct::ViaductBuilder::<DummyRpcChildToParent, DummyRequestChildToParent, DummyRpcParentToChild, DummyRequestParentToChild>::child_with_args(
		)
	} {
		// We're the parent process
		Err(_) => std::thread::Builder::new()
			.name("parent".to_string())
			.spawn(|| {
				println!("parent pid {:?}", std::process::id());

				let ((tx, rx), mut child) = viaduct::ViaductBuilder::<
					DummyRpcParentToChild,
					DummyRequestParentToChild,
					DummyRpcChildToParent,
					DummyRequestChildToParent,
				>::parent(Command::new(std::env::current_exe().unwrap()))
				.unwrap()
				.arg("Viaduct test!")
				.build()
				.unwrap();

				std::thread::Builder::new()
					.name("parent event loop".to_string())
					.spawn(move || {
						rx.run(
							|rpc: DummyRpcChildToParent| {
								assert_eq!(rpc.magic, 321);
								println!("[PARENT] RPC received: {}", rpc.magic);
							},
							|request: DummyRequestChildToParent, tx| {
								assert_eq!(request.magic, 420);
								println!("[PARENT] Request received: {}", request.magic);
								tx.respond(DummyResponseParentToChild { magic: (420, 69) }).unwrap();
							},
						)
						.unwrap();
					})
					.unwrap();

				tx.rpc(DummyRpcParentToChild { magic: 123 }).unwrap();

				let response = tx.request::<DummyResponseChildToParent>(DummyRequestParentToChild { magic: 42 }).unwrap();
				assert_eq!(response.magic, 42069);
				println!("[PARENT] Response received: {:?}", response.magic);

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

					std::thread::Builder::new()
						.name("child event loop".to_string())
						.spawn(move || {
							rx.run(
								|rpc: DummyRpcParentToChild| {
									assert_eq!(rpc.magic, 123);
									println!("[CHILD] RPC received: {}", rpc.magic);
								},
								|request: DummyRequestParentToChild, tx| {
									assert_eq!(request.magic, 42);
									println!("[CHILD] Request received: {}", request.magic);
									tx.respond(DummyResponseChildToParent { magic: 42069 }).unwrap();
								},
							)
							.unwrap();
						})
						.unwrap();

					tx.rpc(DummyRpcChildToParent { magic: 321 }).unwrap();

					let response = tx
						.request::<DummyResponseParentToChild>(DummyRequestChildToParent { magic: 420 })
						.unwrap();
					assert_eq!(response.magic, (420, 69));
					println!("[CHILD] Response received: {:?}", response.magic);
				})
				.unwrap()
		}
	};

	named_thread.join().ok();
}

#[cfg_attr(feature = "speedy", derive(speedy::Writable, speedy::Readable))]
#[cfg_attr(feature = "bincode", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
/// An RPC that is sent from the parent process to the child process.
struct DummyRpcParentToChild {
	magic: u8,
}
#[cfg_attr(feature = "speedy", derive(speedy::Writable, speedy::Readable))]
#[cfg_attr(feature = "bincode", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
/// An RPC that is sent from the child process to the parent process.
struct DummyRpcChildToParent {
	magic: u16,
}

#[cfg_attr(feature = "speedy", derive(speedy::Writable, speedy::Readable))]
#[cfg_attr(feature = "bincode", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
/// A request that is sent from the parent process to the child process.
struct DummyRequestParentToChild {
	magic: u32,
}
#[cfg_attr(feature = "speedy", derive(speedy::Writable, speedy::Readable))]
#[cfg_attr(feature = "bincode", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
/// A request that is sent from the child process to the parent process.
struct DummyRequestChildToParent {
	magic: u64,
}

#[cfg_attr(feature = "speedy", derive(speedy::Writable, speedy::Readable))]
#[cfg_attr(feature = "bincode", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
/// A response that is sent from the child process to the parent process.
struct DummyResponseChildToParent {
	magic: u128,
}
#[cfg_attr(feature = "speedy", derive(speedy::Writable, speedy::Readable))]
#[cfg_attr(feature = "bincode", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
/// A response that is sent from the parent process to the child process.
struct DummyResponseParentToChild {
	magic: (u128, u8),
}

// Manual serialization and deserialization implementations
#[cfg(not(any(feature = "bincode", feature = "speedy")))]
use std::io::Write;

#[cfg(not(any(feature = "bincode", feature = "speedy")))]
impl viaduct::ViaductSerialize for DummyRpcParentToChild {
	type Error = std::convert::Infallible;

	fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
		buf.write_all(&[self.magic]).unwrap();
		Ok(())
	}
}
#[cfg(not(any(feature = "bincode", feature = "speedy")))]
impl viaduct::ViaductDeserialize for DummyRpcParentToChild {
	type Error = std::convert::Infallible;

	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
		assert_eq!(bytes.len(), 1);
		Ok(Self { magic: bytes[0] })
	}
}
#[cfg(not(any(feature = "bincode", feature = "speedy")))]
impl viaduct::ViaductSerialize for DummyRpcChildToParent {
	type Error = std::convert::Infallible;

	fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
		buf.write_all(&self.magic.to_ne_bytes()).unwrap();
		Ok(())
	}
}
#[cfg(not(any(feature = "bincode", feature = "speedy")))]
impl viaduct::ViaductDeserialize for DummyRpcChildToParent {
	type Error = std::convert::Infallible;

	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
		Ok(Self {
			magic: u16::from_ne_bytes(bytes.try_into().unwrap()),
		})
	}
}
#[cfg(not(any(feature = "bincode", feature = "speedy")))]
impl viaduct::ViaductSerialize for DummyRequestParentToChild {
	type Error = std::convert::Infallible;

	fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
		buf.write_all(&self.magic.to_ne_bytes()).unwrap();
		Ok(())
	}
}
#[cfg(not(any(feature = "bincode", feature = "speedy")))]
impl viaduct::ViaductDeserialize for DummyRequestParentToChild {
	type Error = std::convert::Infallible;

	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
		Ok(Self {
			magic: u32::from_ne_bytes(bytes.try_into().unwrap()),
		})
	}
}
#[cfg(not(any(feature = "bincode", feature = "speedy")))]
impl viaduct::ViaductSerialize for DummyRequestChildToParent {
	type Error = std::convert::Infallible;

	fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
		buf.write_all(&self.magic.to_ne_bytes()).unwrap();
		Ok(())
	}
}
#[cfg(not(any(feature = "bincode", feature = "speedy")))]
impl viaduct::ViaductDeserialize for DummyRequestChildToParent {
	type Error = std::convert::Infallible;

	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
		Ok(Self {
			magic: u64::from_ne_bytes(bytes.try_into().unwrap()),
		})
	}
}
#[cfg(not(any(feature = "bincode", feature = "speedy")))]
impl viaduct::ViaductSerialize for DummyResponseChildToParent {
	type Error = std::convert::Infallible;

	fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
		buf.write_all(&self.magic.to_ne_bytes()).unwrap();
		Ok(())
	}
}
#[cfg(not(any(feature = "bincode", feature = "speedy")))]
impl viaduct::ViaductDeserialize for DummyResponseChildToParent {
	type Error = std::convert::Infallible;

	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
		Ok(Self {
			magic: u128::from_ne_bytes(bytes.try_into().unwrap()),
		})
	}
}
#[cfg(not(any(feature = "bincode", feature = "speedy")))]
impl viaduct::ViaductSerialize for DummyResponseParentToChild {
	type Error = std::convert::Infallible;

	fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
		buf.write_all(&self.magic.0.to_ne_bytes()).unwrap();
		buf.write_all(&self.magic.1.to_ne_bytes()).unwrap();
		Ok(())
	}
}
#[cfg(not(any(feature = "bincode", feature = "speedy")))]
impl viaduct::ViaductDeserialize for DummyResponseParentToChild {
	type Error = std::convert::Infallible;

	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
		Ok(Self {
			magic: (u128::from_ne_bytes(bytes[0..16].try_into().unwrap()), bytes[16]),
		})
	}
}
