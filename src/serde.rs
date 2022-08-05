/// Types that can be serialized and deserialized for crossing the viaduct.
pub trait Pipeable: Sized {
	/// Serialize this type into the given buffer.
	///
	/// # Panics
	///
	/// This function will panic if serialization fails.
	fn to_pipeable(&self, buf: &mut Vec<u8>);

	/// Deserialize this type from the given slice.
	///
	/// # Panics
	///
	/// This function will panic if deserialization fails.
	fn from_pipeable(bytes: &[u8]) -> Self;
}

#[cfg(feature = "bincode")]
impl<T: serde::Serialize + serde::de::DeserializeOwned> Pipeable for T {
	fn to_pipeable(&self, mut buf: &mut Vec<u8>) {
		bincode::serialize_into(&mut buf, self).expect("Failed to serialize value");
	}

	fn from_pipeable(bytes: &[u8]) -> Self {
		bincode::deserialize(bytes).expect("Failed to deserialize value")
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
	fn to_pipeable(&self, mut buf: &mut Vec<u8>) {
		self.write_to_stream(&mut buf).expect("Failed to serialize value");
	}

	fn from_pipeable(bytes: &[u8]) -> Self {
		Self::read_from_buffer_copying_data(bytes).expect("Failed to deserialize value")
	}
}
