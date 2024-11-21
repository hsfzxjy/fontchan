#[cfg(not(target_family = "wasm"))]
#[inline]
pub fn unreachable() -> ! {
    unreachable!()
}

#[cfg(target_family = "wasm")]
#[inline]
pub fn unreachable() -> ! {
    core::arch::wasm32::unreachable()
}
