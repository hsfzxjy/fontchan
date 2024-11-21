#[cfg(not(feature = "has-std"))]
#[inline]
/// No panic version
pub fn split_at(slice: &[u8], index: usize) -> (&[u8], &[u8]) {
    unsafe {
        let ptr = slice.as_ptr();
        (
            core::slice::from_raw_parts(ptr, index),
            core::slice::from_raw_parts(ptr.add(index), slice.len() - index),
        )
    }
}

#[cfg(feature = "has-std")]
pub fn split_at(slice: &[u8], index: usize) -> (&[u8], &[u8]) {
    slice.split_at(index)
}

#[cfg(not(feature = "has-std"))]
#[inline]
pub fn split_at_first(slice: &[u8]) -> (u8, &[u8]) {
    let first = unsafe { *slice.get_unchecked(0) };
    (first, split_at(slice, 1).1)
}

#[cfg(feature = "has-std")]
pub fn split_at_first(slice: &[u8]) -> (u8, &[u8]) {
    let (first, rest) = slice.split_first().unwrap();
    (*first, rest)
}
