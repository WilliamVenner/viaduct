use crate::{ViaductRequestResponder, ViaductRx, ViaductTx};
use std::fmt::Debug;

impl<A, B, C, D> Debug for ViaductRequestResponder<A, B, C, D> {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ViaductRequestResponder").finish()
	}
}

impl<A, B, C, D> Debug for ViaductTx<A, B, C, D> {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ViaductTx").finish()
	}
}

impl<A, B, C, D> Debug for ViaductRx<A, B, C, D> {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ViaductRx").finish()
	}
}
