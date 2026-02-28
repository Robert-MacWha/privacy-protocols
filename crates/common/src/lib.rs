/// Conditional Send + Sync bound: required on native, no-op on WASM.
#[cfg(not(feature = "wasm"))]
pub trait MaybeSend: Send + Sync {}
#[cfg(not(feature = "wasm"))]
impl<T: Send + Sync> MaybeSend for T {}

#[cfg(feature = "wasm")]
pub trait MaybeSend {}
#[cfg(feature = "wasm")]
impl<T> MaybeSend for T {}
