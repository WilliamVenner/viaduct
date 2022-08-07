/// Types that can be serialized and deserialized for crossing the viaduct.
pub trait ViaductSerialize {
	/// The error returned if we fail to serialize the data.
	type Error: std::fmt::Debug;

	/// Serialize this type into the given buffer.
	///
	/// The buffer will be empty when this function is called. Try not to fiddle with the capacity of the buffer, as it will be reused.
	fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error>;
}

/// Types that can be serialized and deserialized for crossing the viaduct.
pub trait ViaductDeserialize: Sized {
	/// The error returned if we fail to deserialize the data.
	type Error: std::fmt::Debug;

	/// Deserialize this type from the given slice.
	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error>;
}

#[derive(Clone, Copy, Debug)]
/// You can use this type (which implements [`ViaductSerialize`] and [`ViaductDeserialize`]) to specify that this type of packet (RCP/request) will never happen.
pub enum Never {}
impl ViaductSerialize for Never {
	type Error = std::convert::Infallible;

	fn to_pipeable(&self, _buf: &mut Vec<u8>) -> Result<(), Self::Error> {
		unreachable!()
	}
}
impl ViaductDeserialize for Never {
	type Error = std::convert::Infallible;

	fn from_pipeable(_bytes: &[u8]) -> Result<Self, Self::Error> {
		unreachable!()
	}
}

#[cfg(feature = "bincode")]
mod bincode {
	impl<T: serde::Serialize> ViaductSerialize for T {
		type Error = bincode::Error;

		#[inline]
		fn to_pipeable(&self, mut buf: &mut Vec<u8>) -> Result<(), Self::Error> {
			bincode::serialize_into(&mut buf, self)
		}
	}
	impl<T: serde::de::DeserializeOwned> ViaductDeserialize for T {
		type Error = bincode::Error;

		#[inline]
		fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
			bincode::deserialize(bytes)
		}
	}
}

#[cfg(feature = "speedy")]
mod speedy {
	#[cfg(target_endian = "little")]
	type SpeedyEndian = speedy::LittleEndian;

	#[cfg(target_endian = "big")]
	type SpeedyEndian = speedy::BigEndian;

	impl<T: speedy::Writable<SpeedyEndian>> ViaductSerialize for T {
		type Error = speedy::Error;

		#[inline]
		fn to_pipeable(&self, mut buf: &mut Vec<u8>) -> Result<(), Self::Error> {
			self.write_to_stream(&mut buf)
		}
	}
	impl<'de, T: speedy::Writable<SpeedyEndian>> ViaductDeserialize for T {
		type Error = speedy::Error;

		#[inline]
		fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
			Self::read_from_buffer_copying_data(bytes)
		}
	}
}

#[cfg(not(any(feature = "bincode", feature = "speedy")))]
mod primitives {
	use super::{ViaductSerialize, ViaductDeserialize};
	use std::mem::size_of;

	impl ViaductSerialize for () {
		type Error = std::convert::Infallible;

		#[inline]
		fn to_pipeable(&self, _buf: &mut Vec<u8>) -> Result<(), Self::Error> {
			Ok(())
		}
	}
	impl ViaductDeserialize for () {
		type Error = &'static str;

		#[inline]
		fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
			if bytes.is_empty() {
				Ok(())
			} else {
				Err("expected empty slice")
			}
		}
	}

	impl ViaductSerialize for u8 {
		type Error = &'static str;

		#[inline]
		fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
			buf.push(*self);
			Ok(())
		}
	}
	impl ViaductDeserialize for u8 {
		type Error = &'static str;

		#[inline]
		fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
			if bytes.len() == 1 {
				Ok(bytes[0])
			} else {
				Err("empty slice")
			}
		}
	}

	impl ViaductSerialize for i8 {
		type Error = &'static str;

		#[inline]
		fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
			buf.push(*self as u8);
			Ok(())
		}
	}
	impl ViaductDeserialize for i8 {
		type Error = &'static str;

		#[inline]
		fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
			if bytes.len() == 1 {
				Ok(bytes[0] as i8)
			} else {
				Err("empty slice")
			}
		}
	}

	impl ViaductSerialize for bool {
		type Error = &'static str;

		#[inline]
		fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
			buf.push(*self as u8);
			Ok(())
		}
	}
	impl ViaductDeserialize for bool {
		type Error = &'static str;

		#[inline]
		fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
			if bytes.len() == 1 {
				match bytes[0] {
					0 => Ok(false),
					1 => Ok(true),
					_ => Err("invalid bool"),
				}
			} else {
				Err("empty slice")
			}
		}
	}

	impl ViaductSerialize for char {
		type Error = std::convert::Infallible;

		#[inline]
		fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
			(*self as u32).to_pipeable(buf)
		}
	}
	impl ViaductDeserialize for char {
		type Error = &'static str;

		#[inline]
		fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
			char::from_u32(u32::from_pipeable(bytes).map_err(|_| "could not convert slice to array")?).ok_or("invalid char")
		}
	}

	macro_rules! impl_for_primitive {
		{$($ty:ty)*} => {$(
			impl ViaductSerialize for $ty {
				type Error = std::convert::Infallible;

				#[inline]
				fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
					buf.extend_from_slice(&self.to_ne_bytes());
					Ok(())
				}
			}
			impl ViaductDeserialize for $ty {
				type Error = std::array::TryFromSliceError;

				#[inline]
				fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
					let bytes: [u8; size_of::<$ty>()] = bytes.try_into()?;
					Ok(<$ty>::from_ne_bytes(bytes))
				}
			}
		)*};
	}
	impl_for_primitive! { u16 u32 u64 u128 usize i16 i32 i64 i128 isize f32 f64 }

	impl ViaductSerialize for str {
		type Error = std::convert::Infallible;

		#[inline]
		fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
			buf.extend_from_slice(&usize::to_ne_bytes(self.len()));
			buf.extend_from_slice(self.as_bytes());
			Ok(())
		}
	}
	impl ViaductDeserialize for String {
		type Error = &'static str;

		fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
			let vec = Vec::from_pipeable(bytes)?;
			String::from_utf8(vec).map_err(|_| "invalid Unicode")
		}
	}

	impl ViaductSerialize for [u8] {
		type Error = std::convert::Infallible;

		#[inline]
		fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
			buf.extend_from_slice(&usize::to_ne_bytes(self.len()));
			buf.extend_from_slice(self);
			Ok(())
		}
	}
	impl ViaductDeserialize for Vec<u8> {
		type Error = &'static str;

		fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
			let len = {
				let mut len = [0u8; size_of::<usize>()];
				len.copy_from_slice(&bytes[..size_of::<usize>()]);
				usize::from_ne_bytes(len)
			};
			if bytes.len() - size_of::<usize>() < len {
				return Err("not enough bytes for slice");
			}
			Ok(bytes[size_of::<usize>()..].to_vec())
		}
	}
	macro_rules! impl_for_slice {
		($($ty:ty)*) => {$(
			impl<const N: usize> ViaductSerialize for [$ty; N] {
				type Error = std::convert::Infallible;

				#[inline]
				fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
					self.as_slice().to_pipeable(buf)
				}
			}
			impl<const N: usize> ViaductDeserialize for [$ty; N] {
				type Error = &'static str;

				fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
					if bytes.len() != N * size_of::<$ty>() {
						return Err("mismatched length for slice");
					}

					todo!()
				}
			}

			impl ViaductSerialize for [$ty] {
				type Error = std::convert::Infallible;

				#[inline]
				fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
					buf.extend_from_slice(&usize::to_ne_bytes(self.len()));
					for value in self.iter().copied() {
						buf.extend_from_slice(&value.to_ne_bytes());
					}
					Ok(())
				}
			}
			impl ViaductDeserialize for Vec<$ty> {
				type Error = &'static str;

				fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
					let len = {
						let mut len = [0u8; size_of::<usize>()];
						len.copy_from_slice(&bytes[..size_of::<usize>()]);
						usize::from_ne_bytes(len)
					};
					if bytes.len() - size_of::<usize>() != len * size_of::<$ty>() {
						return Err("mismatched length for slice");
					}

					let mut vec = Vec::with_capacity(len);

					let (prefix, middle, suffix) = unsafe { bytes.align_to::<$ty>() };

					for chunk in prefix.chunks_exact(size_of::<$ty>()) {
						vec.push(<$ty>::from_ne_bytes(chunk.try_into().unwrap()));
					}

					vec.extend_from_slice(middle);
					for chunk in suffix.chunks_exact(size_of::<$ty>()) {
						vec.push(<$ty>::from_ne_bytes(chunk.try_into().unwrap()));
					}

					Ok(vec)
				}
			}
		)*};
	}
	impl_for_slice! { u16 u32 u64 u128 usize i16 i32 i64 i128 isize f32 f64 }

	#[test]
	fn test_serde_primitives() {
		use rand::prelude::*;

		let mut rand = rand::thread_rng();

		let mut buffer = Vec::new();

		macro_rules! test_serde_primitives {
			($($ty:ty)*) => {$({
				buffer.clear();
				let value: $ty = rand.gen();
				value.to_pipeable(&mut buffer).unwrap();
				assert_eq!(value, <$ty>::from_pipeable(&buffer).unwrap());

				// TODO test vec
				// TODO test array
			})*};
		}
		test_serde_primitives! { () u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 bool char }

		"Hello, world!".to_pipeable(&mut buffer).unwrap();
		assert_eq!("Hello, world!", String::from_pipeable(&buffer).unwrap());

		// TODO std::collections
	}
}