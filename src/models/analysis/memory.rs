// Copyright (C) Microsoft Corporation. All rights reserved.

#![deny(clippy::integer_arithmetic)]

use core::ops::{Add, AddAssign, Sub, SubAssign};
use num_traits::{CheckedAdd, CheckedSub, WrappingAdd, WrappingSub};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter, Result};

/// Virtual Memory Address
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default, JsonSchema,
)]
pub struct VirtualAddress(pub u64);

impl VirtualAddress {
    /// True if the `VirtualAddress` is a nullptr
    #[must_use]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Checks if this is a Kernel Virtual Address
    ///
    /// Returns true if the address is in the higher canonical address range
    #[must_use]
    pub const fn is_kernel_space(&self) -> bool {
        self.0 & 0xffff_8000_0000_0000 == 0xffff_8000_0000_0000
    }

    /// Construct a `VirtualAddress` from an address value in memory
    #[must_use]
    pub const fn from_le_bytes(bytes: [u8; 8]) -> Self {
        Self(u64::from_le_bytes(bytes))
    }
}

impl WrappingAdd for VirtualAddress {
    fn wrapping_add(&self, v: &Self) -> Self {
        Self(self.0.wrapping_add(v.0))
    }
}

impl WrappingSub for VirtualAddress {
    fn wrapping_sub(&self, v: &Self) -> Self {
        Self(self.0.wrapping_sub(v.0))
    }
}

impl CheckedAdd for VirtualAddress {
    fn checked_add(&self, v: &Self) -> Option<Self> {
        self.0.checked_add(v.0).map(Self)
    }
}

impl CheckedSub for VirtualAddress {
    fn checked_sub(&self, v: &Self) -> Option<Self> {
        self.0.checked_sub(v.0).map(Self)
    }
}

// Only concerned with 64-bit addresses
impl From<[u8; 8]> for VirtualAddress {
    fn from(v: [u8; 8]) -> Self {
        VirtualAddress(u64::from_le_bytes(v))
    }
}

/// conversion implementations for common unsigned types
macro_rules! uint_impl {
    ($t:ty) => {
        impl From<$t> for VirtualAddress {
            fn from(value: $t) -> Self {
                Self(value.into())
            }
        }
    };
}

/// conversion implementations for common signed types
macro_rules! int_impl {
    ($t:ty) => {
        impl From<$t> for VirtualAddress {
            fn from(value: $t) -> Self {
                0_u64.wrapping_add_signed(value.into()).into()
            }
        }
    };
}

uint_impl!(u8);
uint_impl!(u16);
uint_impl!(u32);
uint_impl!(u64);
int_impl!(i8);
int_impl!(i16);
int_impl!(i32);
int_impl!(i64);

impl<T> Add<T> for VirtualAddress
where
    T: Into<VirtualAddress>,
{
    type Output = VirtualAddress;
    // Explicitly wrap additions by design
    fn add(self, value: T) -> Self::Output {
        self.wrapping_add(&value.into())
    }
}

impl<T> AddAssign<T> for VirtualAddress
where
    T: Into<VirtualAddress>,
{
    fn add_assign(&mut self, rhs: T) {
        *self = self.wrapping_add(&rhs.into());
    }
}

impl<T> SubAssign<T> for VirtualAddress
where
    T: Into<VirtualAddress>,
{
    fn sub_assign(&mut self, rhs: T) {
        *self = self.wrapping_sub(&rhs.into());
    }
}

impl<I> Sub<I> for VirtualAddress
where
    I: Into<VirtualAddress>,
{
    type Output = VirtualAddress;
    // Explicitly wrap subtractions by design
    fn sub(self, rhs: I) -> Self::Output {
        self.wrapping_sub(&rhs.into())
    }
}

impl From<VirtualAddress> for usize {
    #[allow(clippy::cast_possible_truncation)]
    fn from(value: VirtualAddress) -> Self {
        value.0 as usize
    }
}

impl From<VirtualAddress> for u64 {
    fn from(value: VirtualAddress) -> Self {
        value.0
    }
}

impl Display for VirtualAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "0x{:x}", self.0)
    }
}

// Custom debug to print address in hex
impl Debug for VirtualAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "VirtualAddress(0x{:x})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let a = VirtualAddress::from(1_u64);
        assert_eq!(a, 1_u32.into());

        let b = VirtualAddress::from(0xFFFF_FFFF_FFFF_FFFE_u64) + 3;
        assert_eq!(a, b);
    }

    #[test]
    fn signed() {
        let mut a = VirtualAddress::from(10_i32);
        a -= -10_i64;
        a += 5_u8;
        let b = VirtualAddress::from(25_u32);
        assert_eq!(a, b);
    }
}
