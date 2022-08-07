//! Types used in doctests for Viaduct.

#![allow(unused)]

#[derive(Debug)]
pub enum ExampleRpc {
	Cow,
	Pig,
	Horse,
}

#[derive(Debug)]
pub enum ExampleRequest {
	DoAFrontflip,
	DoABackflip,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Result<T, E> {
	Ok(T),
	Err(E),
}
pub use self::Result::{Err, Ok};

#[derive(Debug, PartialEq, Eq)]
pub struct FrontflipError;

#[derive(Debug, PartialEq, Eq)]
pub struct BackflipError;

impl<T, E> super::serde::Pipeable for Result<T, E> {
	type SerializeError = std::convert::Infallible;
	type DeserializeError = std::convert::Infallible;

	fn to_pipeable(&self, buf: &mut Vec<u8>) -> std::result::Result<(), Self::SerializeError> {
		unimplemented!()
	}

	fn from_pipeable(bytes: &[u8]) -> std::result::Result<Self, Self::DeserializeError> {
		unimplemented!()
	}
}

impl super::serde::Pipeable for ExampleRpc {
	type SerializeError = std::convert::Infallible;
	type DeserializeError = std::convert::Infallible;

	fn to_pipeable(&self, buf: &mut Vec<u8>) -> std::result::Result<(), Self::SerializeError> {
		unimplemented!()
	}

	fn from_pipeable(bytes: &[u8]) -> std::result::Result<Self, Self::DeserializeError> {
		unimplemented!()
	}
}

impl super::serde::Pipeable for ExampleRequest {
	type SerializeError = std::convert::Infallible;
	type DeserializeError = std::convert::Infallible;

	fn to_pipeable(&self, buf: &mut Vec<u8>) -> std::result::Result<(), Self::SerializeError> {
		unimplemented!()
	}

	fn from_pipeable(bytes: &[u8]) -> std::result::Result<Self, Self::DeserializeError> {
		unimplemented!()
	}
}
