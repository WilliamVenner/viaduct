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
pub struct FrontflipError;
impl super::ViaductSerialize for FrontflipError {
	type Error = std::convert::Infallible;

	#[inline]
	fn to_pipeable(&self, buf: &mut Vec<u8>) -> std::result::Result<(), Self::Error> {
		unimplemented!()
	}
}
impl super::ViaductDeserialize for FrontflipError {
	type Error = std::convert::Infallible;

	#[inline]
	fn from_pipeable(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
		unimplemented!()
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct BackflipError;
impl super::ViaductSerialize for BackflipError {
	type Error = std::convert::Infallible;

	#[inline]
	fn to_pipeable(&self, buf: &mut Vec<u8>) -> std::result::Result<(), Self::Error> {
		unimplemented!()
	}
}
impl super::ViaductDeserialize for BackflipError {
	type Error = std::convert::Infallible;

	#[inline]
	fn from_pipeable(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
		unimplemented!()
	}
}

impl super::ViaductSerialize for ExampleRpc {
	type Error = std::convert::Infallible;

	#[inline]
	fn to_pipeable(&self, buf: &mut Vec<u8>) -> std::result::Result<(), Self::Error> {
		unimplemented!()
	}
}
impl super::ViaductDeserialize for ExampleRpc {
	type Error = std::convert::Infallible;

	#[inline]
	fn from_pipeable(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
		unimplemented!()
	}
}

impl super::ViaductSerialize for ExampleRequest {
	type Error = std::convert::Infallible;

	#[inline]
	fn to_pipeable(&self, buf: &mut Vec<u8>) -> std::result::Result<(), Self::Error> {
		unimplemented!()
	}
}
impl super::ViaductDeserialize for ExampleRequest {
	type Error = std::convert::Infallible;

	#[inline]
	fn from_pipeable(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
		unimplemented!()
	}
}
