//! Types used in doctests for Viaduct.

#![allow(unused)]

pub enum ExampleRpc { Cow, Pig, Horse }
pub enum ExampleRequest { DoAFrontflip, DoABackflip }
pub enum ExampleResponse { FrontflipOk, BackflipOk }

#[derive(Debug, PartialEq, Eq)]
pub enum Result<T, E> {
	Ok(T),
	Err(E)
}
pub use self::Result::{Ok, Err};

#[derive(Debug, PartialEq, Eq)]
pub struct FrontflipError;

#[derive(Debug, PartialEq, Eq)]
pub struct BackflipError;

impl<T, E> super::serde::Pipeable for Result<T, E> {
    fn to_pipeable(&self, buf: &mut Vec<u8>) {
        unimplemented!()
    }

    fn from_pipeable(bytes: &[u8]) -> Self {
        unimplemented!()
    }
}

impl super::serde::Pipeable for ExampleRpc {
    fn to_pipeable(&self, buf: &mut Vec<u8>) {
        unimplemented!()
    }

    fn from_pipeable(bytes: &[u8]) -> Self {
        unimplemented!()
    }
}

impl super::serde::Pipeable for ExampleRequest {
    fn to_pipeable(&self, buf: &mut Vec<u8>) {
        unimplemented!()
    }

    fn from_pipeable(bytes: &[u8]) -> Self {
        unimplemented!()
    }
}

impl super::serde::Pipeable for ExampleResponse {
    fn to_pipeable(&self, buf: &mut Vec<u8>) {
        unimplemented!()
    }

    fn from_pipeable(bytes: &[u8]) -> Self {
        unimplemented!()
    }
}