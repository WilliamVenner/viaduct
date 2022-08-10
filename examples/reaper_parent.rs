use parking_lot::Mutex;
use std::{process::Command, sync::Arc, time::Duration};
use viaduct::{Never, ViaductChild, ViaductParent};

fn main() {
	if let Ok(_child) = unsafe { ViaductChild::<Never, Never, Never, Never>::new().build() } {
		println!("[CHILD] Exiting in 5 seconds...");
		std::thread::sleep(Duration::from_secs(5));
		println!("[CHILD] Goodbye!");
	// exiting...
	} else {
		let shared_child = Arc::new(Mutex::new(None::<std::process::Child>));
		let shared_child_ref = shared_child.clone();

		let (_, child) = ViaductParent::<Never, Never, Never, Never>::new(Command::new(std::env::current_exe().unwrap()))
			.unwrap()
			.with_reaper(move || {
				std::thread::sleep(Duration::from_secs(1));
				match shared_child_ref.lock().take().map(|mut child| child.try_wait()) {
					Some(Ok(None)) => panic!("[PARENT] Child process exited too early"),
					_ => {
						println!("[PARENT] Reaper callback!");
						std::process::exit(0)
					}
				}
			})
			.build()
			.unwrap();

		*shared_child.lock() = Some(child);

		std::thread::park_timeout(Duration::from_secs(30));
		println!("[PARENT] Reaper callback failed");
		std::process::exit(1);
	}
}
