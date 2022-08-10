use crate::os::RawPipe;
use interprocess::unnamed_pipe::{UnnamedPipeReader, UnnamedPipeWriter};
use std::{
	io::{Read, Write},
	time::Duration,
};

pub(super) type ReaperCallbackFn = Box<dyn FnOnce() + Send + 'static>;

pub(super) struct DroppablePipe<Pipe: RawPipe>(Option<Pipe>);
impl<Pipe: RawPipe> DroppablePipe<Pipe> {
	#[inline]
	pub fn new(pipe: Pipe) -> Self {
		Self(Some(pipe))
	}
}
impl<Pipe: RawPipe> RawPipe for DroppablePipe<Pipe> {
	type Raw = Pipe::Raw;

	fn raw(mut self) -> Self::Raw {
		self.0.take().unwrap().raw()
	}

	fn as_raw(&self) -> Self::Raw {
		self.0.as_ref().unwrap().as_raw()
	}

	fn close(mut self) {
		self.0.take().unwrap().close()
	}

	unsafe fn from_raw(raw: Self::Raw) -> Self {
		Self(Some(unsafe { Pipe::from_raw(raw) }))
	}
}
impl<Pipe: RawPipe> Drop for DroppablePipe<Pipe> {
	fn drop(&mut self) {
		if let Some(r) = self.0.take() {
			r.close();
		}
	}
}
impl<Pipe: RawPipe + Write> Write for DroppablePipe<Pipe> {
	#[inline]
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		if let Some(w) = self.0.as_mut() {
			w.write(buf)
		} else {
			Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
		}
	}

	#[inline]
	fn flush(&mut self) -> std::io::Result<()> {
		if let Some(w) = self.0.as_mut() {
			w.flush()
		} else {
			Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
		}
	}
}
impl<Pipe: RawPipe + Read> Read for DroppablePipe<Pipe> {
	#[inline]
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		if let Some(r) = self.0.as_mut() {
			r.read(buf)
		} else {
			Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
		}
	}
}

pub(crate) unsafe fn child(mut reaper_pipe: DroppablePipe<UnnamedPipeReader>, callback: ReaperCallbackFn) {
	std::thread::spawn(move || {
		loop {
			match reaper_pipe.read(&mut [0]) {
				Ok(0) | Err(_) => break,
				_ => std::thread::sleep(Duration::from_secs(5)),
			}
		}
		callback();
	});
}

pub(crate) unsafe fn parent(mut reaper_pipe: DroppablePipe<UnnamedPipeWriter>, callback: ReaperCallbackFn) {
	std::thread::spawn(move || {
		loop {
			match reaper_pipe.write(&[0]) {
				Ok(0) | Err(_) => break,
				_ => std::thread::sleep(Duration::from_secs(5)),
			}
		}
		callback();
	});
}
