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

#[cfg(all(feature = "bytemuck", not(any(feature = "bincode", feature = "speedy"))))]
mod primitives {
	use super::{ViaductDeserialize, ViaductSerialize};

	impl<T: bytemuck::Pod> ViaductSerialize for T {
		type Error = bytemuck::PodCastError;

		fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::Error> {
			buf.extend_from_slice(bytemuck::bytes_of(self));
			Ok(())
		}
	}

	impl<T: bytemuck::Pod> ViaductDeserialize for T {
		type Error = bytemuck::PodCastError;

		fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::Error> {
			bytemuck::try_from_bytes(bytes).copied()
		}
	}
}
