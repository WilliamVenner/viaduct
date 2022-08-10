use std::{process::Command, time::Duration};
use viaduct::{Never, ViaductChild, ViaductParent};

fn main() {
	let child = unsafe {
		ViaductChild::<Never, Never, Never, Never>::new()
			.with_reaper(|| {
				println!("[CHILD] Reaper callback!");
				std::process::exit(0)
			})
			.build()
	};

	if let Ok(_child) = child {
		println!("[CHILD] Waiting for parent process to exit...");
		std::thread::park_timeout(Duration::from_secs(30));
		println!("[CHILD] Reaper callback failed");
		std::process::exit(1);
	} else {
		let (_, mut child) = std::thread::spawn(|| {
			ViaductParent::<Never, Never, Never, Never>::new(Command::new(std::env::current_exe().unwrap()))
				.unwrap()
				.build()
				.unwrap()
		})
		.join()
		.unwrap();

		if child.try_wait().unwrap().is_some() {
			panic!("[PARENT] Child process exited too early");
		}

		println!("[PARENT] Exiting in 5 seconds...");
		std::thread::sleep(Duration::from_secs(5));
		// exiting...
		println!("[PARENT] Goodbye!");
	}
}
