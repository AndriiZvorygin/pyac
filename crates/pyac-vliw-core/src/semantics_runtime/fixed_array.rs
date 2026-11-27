//! Fixed-size array access, forwarded through references.
//!
//! The `include!`d file is generated at build time and lists the array shapes the
//! runtime and the GPU kernels index.

#![allow(dead_code)]
#![allow(unsafe_code)]

#[cfg_attr(not(target_arch = "spirv"), allow(dead_code))]
pub(crate) trait FixedArrayAccess {
    type Item: Copy;

    fn first(&self) -> Self::Item;
    fn read_at(&self, index: usize, output: &mut Self::Item) -> bool;
    fn write_at(&mut self, index: usize, value: Self::Item) -> bool;
}

impl<A: FixedArrayAccess + ?Sized> FixedArrayAccess for &A {
    type Item = A::Item;

    fn first(&self) -> Self::Item {
        (**self).first()
    }

    fn read_at(&self, index: usize, output: &mut Self::Item) -> bool {
        (**self).read_at(index, output)
    }

    fn write_at(&mut self, _index: usize, _value: Self::Item) -> bool {
        false
    }
}

impl<A: FixedArrayAccess + ?Sized> FixedArrayAccess for &mut A {
    type Item = A::Item;

    fn first(&self) -> Self::Item {
        (**self).first()
    }

    fn read_at(&self, index: usize, output: &mut Self::Item) -> bool {
        (**self).read_at(index, output)
    }

    fn write_at(&mut self, index: usize, value: Self::Item) -> bool {
        (**self).write_at(index, value)
    }
}

include!(concat!(env!("OUT_DIR"), "/fixed_array_access.rs"));
