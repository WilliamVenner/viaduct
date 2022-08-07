/// Types that can be serialized and deserialized for crossing the viaduct.
pub trait Pipeable: Sized {
	/// The error returned if we fail to serialize the data.
	type SerializeError: std::fmt::Debug;

	/// The error returned if we fail to deserialize the data.
	type DeserializeError: std::fmt::Debug;

	/// Serialize this type into the given buffer.
	///
	/// The buffer will be empty when this function is called. Try not to fiddle with the capacity of the buffer, as it will be reused.
	fn to_pipeable(&self, buf: &mut Vec<u8>) -> Result<(), Self::SerializeError>;

	/// Deserialize this type from the given slice.
	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::DeserializeError>;
}

#[cfg(feature = "bincode")]
impl<T: serde::Serialize + serde::de::DeserializeOwned> Pipeable for T {
	type SerializeError = bincode::Error;
	type DeserializeError = bincode::Error;

	#[inline]
	fn to_pipeable(&self, mut buf: &mut Vec<u8>) -> Result<(), Self::SerializeError> {
		bincode::serialize_into(&mut buf, self)
	}

	#[inline]
	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::DeserializeError> {
		bincode::deserialize(bytes)
	}
}

#[cfg(feature = "speedy")]
#[cfg(target_endian = "little")]
type SpeedyEndian = speedy::LittleEndian;

#[cfg(feature = "speedy")]
#[cfg(target_endian = "big")]
type SpeedyEndian = speedy::BigEndian;

#[cfg(feature = "speedy")]
impl<'de, T: speedy::Writable<SpeedyEndian> + speedy::Readable<'de, SpeedyEndian>> Pipeable for T {
	type SerializeError = speedy::Error;
	type DeserializeError = speedy::Error;

	#[inline]
	fn to_pipeable(&self, mut buf: &mut Vec<u8>) -> Result<(), Self::SerializeError> {
		self.write_to_stream(&mut buf)
	}

	#[inline]
	fn from_pipeable(bytes: &[u8]) -> Result<Self, Self::DeserializeError> {
		Self::read_from_buffer_copying_data(bytes)
	}
}
