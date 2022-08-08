//! Types used in doctests for Viaduct.

#![allow(unused)]

use crate::{ViaductDeserialize, ViaductSerialize};

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

impl<T, E> ViaductSerialize for self::Result<T, E> {
	type Error = std::convert::Infallible;

	#[inline]
	fn to_pipeable(&self, buf: &mut Vec<u8>) -> std::result::Result<(), Self::Error> {
		unimplemented!()
	}
}
impl<T, E> ViaductDeserialize for self::Result<T, E> {
	type Error = std::convert::Infallible;

	#[inline]
	fn from_pipeable(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
		unimplemented!()
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct FrontflipError;
impl ViaductSerialize for FrontflipError {
	type Error = std::convert::Infallible;

	#[inline]
	fn to_pipeable(&self, buf: &mut Vec<u8>) -> std::result::Result<(), Self::Error> {
		unimplemented!()
	}
}
impl ViaductDeserialize for FrontflipError {
	type Error = std::convert::Infallible;

	#[inline]
	fn from_pipeable(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
		unimplemented!()
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct BackflipError;
impl ViaductSerialize for BackflipError {
	type Error = std::convert::Infallible;

	#[inline]
	fn to_pipeable(&self, buf: &mut Vec<u8>) -> std::result::Result<(), Self::Error> {
		unimplemented!()
	}
}
impl ViaductDeserialize for BackflipError {
	type Error = std::convert::Infallible;

	#[inline]
	fn from_pipeable(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
		unimplemented!()
	}
}

impl ViaductSerialize for ExampleRpc {
	type Error = std::convert::Infallible;

	#[inline]
	fn to_pipeable(&self, buf: &mut Vec<u8>) -> std::result::Result<(), Self::Error> {
		unimplemented!()
	}
}
impl ViaductDeserialize for ExampleRpc {
	type Error = std::convert::Infallible;

	#[inline]
	fn from_pipeable(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
		unimplemented!()
	}
}

impl ViaductSerialize for ExampleRequest {
	type Error = std::convert::Infallible;

	#[inline]
	fn to_pipeable(&self, buf: &mut Vec<u8>) -> std::result::Result<(), Self::Error> {
		unimplemented!()
	}
}
impl ViaductDeserialize for ExampleRequest {
	type Error = std::convert::Infallible;

	#[inline]
	fn from_pipeable(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
		unimplemented!()
	}
}
