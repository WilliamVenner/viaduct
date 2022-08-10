use interprocess::unnamed_pipe::{UnnamedPipeReader, UnnamedPipeWriter};

pub(super) trait RawPipe: Sized {
	type Raw: std::fmt::Debug;
	fn raw(self) -> Self::Raw;
	fn as_raw(&self) -> Self::Raw;
	fn close(self);
	unsafe fn from_raw(raw: Self::Raw) -> Self;
}
#[cfg(windows)]
impl RawPipe for UnnamedPipeReader {
	type Raw = std::os::windows::io::RawHandle;

	fn raw(self) -> Self::Raw {
		use std::os::windows::prelude::IntoRawHandle;
		self.into_raw_handle()
	}

	fn as_raw(&self) -> Self::Raw {
		use std::os::windows::prelude::AsRawHandle;
		self.as_raw_handle()
	}

	fn close(self) {
		use std::os::windows::prelude::IntoRawHandle;
		unsafe { windows::Win32::Foundation::CloseHandle(Some(windows::Win32::Foundation::HANDLE(self.into_raw_handle() as _))) };
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

	fn as_raw(&self) -> Self::Raw {
		use std::os::windows::prelude::AsRawHandle;
		self.as_raw_handle()
	}

	fn close(self) {
		use std::os::windows::prelude::IntoRawHandle;
		unsafe { windows::Win32::Foundation::CloseHandle(Some(windows::Win32::Foundation::HANDLE(self.into_raw_handle() as _))) };
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

	fn as_raw(&self) -> Self::Raw {
		use std::os::unix::prelude::AsRawFd;
		self.as_raw_fd()
	}

	fn close(self) {
		use std::os::unix::prelude::IntoRawFd;
		unsafe { libc::close(self.into_raw_fd()) };
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

	fn as_raw(&self) -> Self::Raw {
		use std::os::unix::prelude::AsRawFd;
		self.as_raw_fd()
	}

	fn close(self) {
		use std::os::unix::prelude::IntoRawFd;
		unsafe { libc::close(self.into_raw_fd()) };
	}

	unsafe fn from_raw(raw: Self::Raw) -> Self {
		use std::os::unix::prelude::FromRawFd;
		unsafe { Self::from_raw_fd(raw) }
	}
}
