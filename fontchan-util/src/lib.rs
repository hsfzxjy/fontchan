mod atomic_path;
mod digest;
mod factory;
mod hkt;
mod lazy_file;
mod quant;
mod routine;
mod work_dir;

use std::{
    borrow::{Borrow, Cow},
    ops::Range,
};

pub use atomic_path::{AtomicPath, WritableAtomicPath};
pub use digest::*;
pub use factory::*;
pub use hkt::*;
pub use lazy_file::LazyFile;
pub use quant::*;
pub use routine::*;
pub use work_dir::*;

mod unstable_port {
    use std::ops::Range;
    pub fn subslice_range<'a, T: 'a>(this: &'a [T], subslice: &'a [T]) -> Option<Range<usize>> {
        if std::mem::size_of::<T>() == 0 {
            panic!("elements are zero-sized");
        }

        let this_start = this.as_ptr() as usize;
        let subslice_start = subslice.as_ptr() as usize;

        let byte_start = subslice_start.wrapping_sub(this_start);

        if byte_start % core::mem::size_of::<T>() != 0 {
            return None;
        }

        let start = byte_start / core::mem::size_of::<T>();
        let end = start.wrapping_add(subslice.len());

        if start <= this.len() && end <= this.len() {
            Some(start..end)
        } else {
            None
        }
    }
}

pub use unstable_port::*;

pub trait StrExt {
    fn get_substr_range(&self, substr: &str) -> Option<Range<usize>>;
}

impl StrExt for str {
    fn get_substr_range(&self, substr: &str) -> Option<Range<usize>> {
        subslice_range(self.as_bytes(), substr.as_bytes())
    }
}

pub trait StringExt {
    fn retain_range(&mut self, range: Range<usize>) -> Option<()>;
}

impl StringExt for String {
    fn retain_range(&mut self, range: Range<usize>) -> Option<()> {
        let Range { start, end } = range;
        if !(start <= end && end <= self.len()) {
            return None;
        }
        let len = end - start;
        if len == 0 {
            self.clear();
            return Some(());
        }
        self.truncate(end);
        drop(self.drain(..start));
        Some(())
    }
}

pub trait CloneS<'a, 'b: 'a> {
    type Output;
    fn clone_s(&'b self) -> Self::Output;
}

impl<'a, 'b: 'a, T: ?Sized + ToOwned + 'b> CloneS<'a, 'b> for Cow<'a, T> {
    type Output = Cow<'b, T>;

    fn clone_s(&'b self) -> Self::Output {
        Cow::Borrowed(&**self)
    }
}

pub trait CowExt<'a: 'b, 'b> {
    type LOutput;
    type StaticOutput;
    fn clone_l(&self) -> Self::LOutput;
    fn into_static(self) -> Self::StaticOutput;
}

impl<'a: 'b, 'b, T: ?Sized + ToOwned + 'static> CowExt<'a, 'b> for Cow<'a, T> {
    type LOutput = Cow<'b, T>;
    type StaticOutput = Cow<'static, T>;
    fn clone_l(&self) -> Cow<'b, T> {
        match self {
            Cow::Borrowed(b) => Cow::Owned((*b).to_owned()),
            Cow::Owned(o) => Cow::Owned(o.borrow().to_owned()),
        }
    }
    fn into_static(self) -> Cow<'static, T> {
        match self {
            Cow::Borrowed(b) => Cow::Owned(b.to_owned()),
            Cow::Owned(o) => Cow::Owned(o),
        }
    }
}

pub mod exts {
    pub use super::CloneS;
    pub use super::CowExt;
    pub use super::StrExt;
    pub use super::StringExt;
}
