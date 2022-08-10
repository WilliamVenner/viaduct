use crate::{ViaductDeserialize, ViaductRequestResponder, ViaductRx, ViaductSerialize, ViaductTx};
use std::fmt::Debug;

impl<RpcTx, RequestTx, RpcRx, RequestRx> Debug for ViaductRequestResponder<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize,
{
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ViaductRequestResponder").finish()
	}
}

impl<RpcTx, RequestTx, RpcRx, RequestRx> Debug for ViaductTx<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize,
{
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ViaductTx").finish()
	}
}

impl<RpcTx, RequestTx, RpcRx, RequestRx> Debug for ViaductRx<RpcTx, RequestTx, RpcRx, RequestRx>
where
	RpcTx: ViaductSerialize,
	RequestTx: ViaductSerialize,
	RpcRx: ViaductDeserialize,
	RequestRx: ViaductDeserialize,
{
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ViaductRx").finish()
	}
}
