use interprocess::unnamed_pipe::{UnnamedPipeReader, UnnamedPipeWriter};

pub(super) trait RawPipe: Sized {
	type Raw;
	fn raw(self) -> Self::Raw;
	unsafe fn from_raw(raw: Self::Raw) -> Self;
}
#[cfg(windows)]
impl RawPipe for UnnamedPipeReader {
	type Raw = std::os::windows::io::RawHandle;

	fn raw(self) -> Self::Raw {
		use std::os::windows::prelude::IntoRawHandle;
		self.into_raw_handle()
	}

	unsafe fn from_raw(raw: Self::Raw) -> Self {
		use std::os::windows::prelude::FromRawHandle;
		unsafe { Self::from_raw_handle(raw) }
	}
}
#[cfg(windows)]
impl RawPipe for UnnamedPipeWriter {
	type Raw = std::os::windows::io::RawHandle;

	fn raw(self) -> Self::Raw {
		use std::os::windows::prelude::IntoRawHandle;
		self.into_raw_handle()
	}

	unsafe fn from_raw(raw: Self::Raw) -> Self {
		use std::os::windows::prelude::FromRawHandle;
		unsafe { Self::from_raw_handle(raw) }
	}
}
#[cfg(unix)]
impl RawPipe for UnnamedPipeReader {
	type Raw = std::os::unix::io::RawFd;

	fn raw(self) -> Self::Raw {
		use std::os::unix::prelude::IntoRawFd;
		self.into_raw_fd()
	}

	unsafe fn from_raw(raw: Self::Raw) -> Self {
		use std::os::unix::prelude::FromRawFd;
		unsafe { Self::from_raw_fd(raw) }
	}
}
#[cfg(unix)]
impl RawPipe for UnnamedPipeWriter {
	type Raw = std::os::unix::io::RawFd;

	fn raw(self) -> Self::Raw {
		use std::os::unix::prelude::IntoRawFd;
		self.into_raw_fd()
	}

	unsafe fn from_raw(raw: Self::Raw) -> Self {
		use std::os::unix::prelude::FromRawFd;
		unsafe { Self::from_raw_fd(raw) }
	}
}
