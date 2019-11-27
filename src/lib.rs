#![feature(type_alias_impl_trait, specialization)]

// TODO: Remove all uses of `Vec`

use fixed::types::{I1F31, U1F31};

pub use derive_more;

pub mod components;
pub mod context;
pub mod octahack_components;
pub mod output;
pub mod params;
pub mod rack;

pub use components::{
    AnyComponent, AnyInputSpec, AnyIter, AnyOutputSpec, AnyParamSpec, Component, GetOutput,
    RuntimeSpecifier, SpecId, Types, ValueIter,
};
pub use context::{GetInput, GetParam};
pub use rack::{ComponentId, Rack, Wire, WireDst, WireSrc};

fn u_to_s(unsigned: U1F31) -> I1F31 {
    I1F31::from_bits(
        unsigned
            .to_bits()
            .wrapping_sub(I1F31::max_value().to_bits() as u32) as _,
    )
}

fn s_to_u(signed: I1F31) -> U1F31 {
    U1F31::from_bits(
        (signed.to_bits() as u32).wrapping_add(I1F31::max_value().to_bits() as u32) as _,
    )
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ValueKind {
    Binary,
    Continuous,
    // The inner U8 is the maximum
    Discrete(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ValueType {
    pub kind: ValueKind,
    pub channels: Option<u8>,
}

impl ValueType {
    pub const fn continuous() -> Self {
        ValueType {
            kind: ValueKind::Continuous,
            channels: None,
        }
    }

    pub const fn mono() -> Self {
        ValueType {
            kind: ValueKind::Continuous,
            channels: Some(1),
        }
    }

    pub const fn stereo() -> Self {
        ValueType {
            kind: ValueKind::Continuous,
            channels: Some(2),
        }
    }
}

pub type Value = I1F31;

pub trait ValueExt {
    fn discrete(self, max: u8) -> u8;
    fn to_u(self) -> U1F31;
    fn from_u(other: U1F31) -> Self;
}

impl ValueExt for Value {
    fn discrete(self, max: u8) -> u8 {
        (f64::from(self) * max as f64) as u8
    }

    fn to_u(self) -> U1F31 {
        s_to_u(self)
    }

    fn from_u(other: U1F31) -> Self {
        u_to_s(other)
    }
}
