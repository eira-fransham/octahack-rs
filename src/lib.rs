#![feature(
    trivial_bounds,
    type_alias_impl_trait,
    specialization,
    never_type,
    exact_size_is_empty,
    generic_associated_types
)]
// TODO: This warning is buggy when used with `feature(trivial_bounds)`
#![allow(trivial_bounds)]
#![deny(unsafe_code)]

use fixed::types::{I1F31, U1F31};
use rand::{rngs::SmallRng, RngCore, SeedableRng};
use std::{
    collections::HashMap,
    fmt,
    hash::{BuildHasherDefault, Hasher},
    num::NonZeroU8,
    ops::{Index, IndexMut},
};

pub use array_iterator;

pub use derive_more;
pub mod components;
pub mod context;
mod display;
pub mod octahack_components;
pub mod output;
pub mod params;
pub mod rack;

pub use components::{
    AnyComponent, AnyInputSpec, AnyOutputSpec, AnyParamSpec, Component, GetOutput,
    RefRuntimeSpecifier, RuntimeSpecifier, SpecId, Types,
};
pub use context::{Context, GetInput, GetParam};
pub use params::DisplayParam;
pub use rack::{Rack, Wire, WireDst, WireSrc};

pub use nom_midi::MidiEventType as MidiValue;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Uid(u32);

impl Uid {
    fn new(inner: u32) -> Self {
        Uid(inner)
    }
}

impl fmt::Display for Uid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

#[derive(Debug)]
pub struct UidGen {
    cur: SmallRng,
}

impl UidGen {
    pub fn new() -> Self {
        UidGen {
            // Since seed doesn't matter (we only ever create one `UidGen` and use it for
            // all `Uid`s) we hard-code it.
            cur: SmallRng::from_seed([0; 16]),
        }
    }

    pub fn next(&mut self) -> Uid {
        let id = self.cur.next_u32();
        Uid(id)
    }
}

#[derive(Clone, Debug)]
pub struct XOrHasher {
    i: SmallRng,
    cur: u64,
}

impl Default for XOrHasher {
    fn default() -> Self {
        XOrHasher {
            i: SmallRng::from_seed([0; 16]),
            cur: 0,
        }
    }
}

impl Hasher for XOrHasher {
    fn finish(&self) -> u64 {
        self.cur
    }

    fn write(&mut self, bytes: &[u8]) {
        for i in bytes.chunks(std::mem::size_of::<u64>()) {
            self.write_u64(
                i.iter()
                    .fold(
                        // left shift, total
                        (0, 0),
                        |(shl, tot), &cur| (shl + 8, tot | (cur as u64) << shl),
                    )
                    .1,
            );
        }
    }

    fn write_u64(&mut self, i: u64) {
        self.cur ^= self.i.next_u64() ^ i;
    }
    fn write_i64(&mut self, i: i64) {
        self.write_u64(i as _)
    }
    fn write_u32(&mut self, i: u32) {
        self.write_u64(i as _)
    }
    fn write_i32(&mut self, i: i32) {
        self.write_u64(i as _)
    }
    fn write_u16(&mut self, i: u16) {
        self.write_u64(i as _)
    }
    fn write_i16(&mut self, i: i16) {
        self.write_u64(i as _)
    }
    fn write_u8(&mut self, i: u8) {
        self.write_u64(i as _)
    }
    fn write_i8(&mut self, i: i8) {
        self.write_u64(i as _)
    }
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as _)
    }
    fn write_isize(&mut self, i: isize) {
        self.write_u64(i as _)
    }
}

#[derive(Debug)]
pub struct UidMap<T> {
    storage: HashMap<Uid, T, BuildHasherDefault<XOrHasher>>,
}

impl<'a, T: 'a> IntoIterator for &'a UidMap<T> {
    type Item = (Uid, &'a T);
    type IntoIter = impl ExactSizeIterator<Item = Self::Item> + 'a;

    fn into_iter(self) -> Self::IntoIter {
        self.storage.iter().map(|(s, v)| (*s, v))
    }
}

impl<T> UidMap<T> {
    pub fn insert(&mut self, uid: Uid, val: T) -> Option<T> {
        self.storage.insert(uid, val)
    }
}

impl<T> Default for UidMap<T> {
    fn default() -> Self {
        UidMap {
            storage: Default::default(),
        }
    }
}

impl<T> Index<&'_ Uid> for UidMap<T> {
    type Output = T;

    fn index(&self, uid: &Uid) -> &Self::Output {
        &self.storage[uid]
    }
}

impl<T> IndexMut<&'_ Uid> for UidMap<T> {
    fn index_mut(&mut self, uid: &Uid) -> &mut Self::Output {
        self.storage.get_mut(uid).unwrap()
    }
}

impl<T> Index<Uid> for UidMap<T> {
    type Output = T;

    fn index(&self, uid: Uid) -> &Self::Output {
        &self[&uid]
    }
}

impl<T> IndexMut<Uid> for UidMap<T> {
    fn index_mut(&mut self, uid: Uid) -> &mut Self::Output {
        &mut self[&uid]
    }
}

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

pub trait UiElement {
    const NAME: &'static str;
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
    pub channels: Option<NonZeroU8>,
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ValueKind::Binary => write!(f, "gate")?,
            ValueKind::Continuous => write!(f, "analogue")?,
            ValueKind::Discrete(max) => write!(f, "discrete(0..{})", max)?,
        }
        match self.channels.map(NonZeroU8::get) {
            Some(1) => {}
            Some(channels) => write!(f, "*{}", channels)?,
            None => write!(f, "*??")?,
        }

        Ok(())
    }
}

// We need `unsafe` because `NonZeroU8::new` isn't a `const fn`
#[allow(unsafe_code)]
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
            channels: Some(unsafe { NonZeroU8::new_unchecked(1) }),
        }
    }

    pub const fn stereo() -> Self {
        ValueType {
            kind: ValueKind::Continuous,
            channels: Some(unsafe { NonZeroU8::new_unchecked(2) }),
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
