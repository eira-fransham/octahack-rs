pub mod anycomponent;

pub use anycomponent::{AnyComponent, AnyInputSpec, AnyOutputSpec, AnyParamSpec, Types};

use crate::{GetInput, GetParam, Value, ValueType};
use itertools::Either;
use nom_midi::MidiEventType;

// TODO: This can probably be `u8`
pub type SpecId = usize;

pub trait Specifier: Sized + Clone + 'static {
    const VALUES: &'static [Self];
    // TODO: This should just be a stopgap until `const fn`s are more fleshed-out - at the
    //       moment it's not possible to define this as `VALUES.map(Self::typeof)`.
    const TYPES: &'static [ValueType];

    fn value_type(&self) -> ValueType {
        Self::TYPES[self.id()]
    }

    fn id(&self) -> SpecId;
    fn from_id(id: SpecId) -> Self {
        Self::VALUES[id].clone()
    }
}

impl Specifier for ! {
    const VALUES: &'static [Self] = &[];
    const TYPES: &'static [ValueType] = &[];

    fn id(&self) -> SpecId {
        unreachable!()
    }
}

pub trait Param: Specifier {
    fn default(&self) -> Value;
}

pub trait Component: Sized {
    type InputSpecifier: Specifier;
    type OutputSpecifier: Specifier;
    type ParamSpecifier: Specifier;
    // TODO: Use GATs to allow adapators to be used internally.
    type OutputIter: ValueIter + Send;

    fn output<Ctx>(&self, id: Self::OutputSpecifier, ctx: Ctx) -> Self::OutputIter
    where
        Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>;

    fn update<Ctx>(&self, _ctx: Ctx) -> Self
    where
        Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>;
}

// TODO: Support MIDI inputs

pub trait ValueIter {
    type Midi: ExactSizeIterator<Item = MidiEventType> + Send;
    type Analog: ExactSizeIterator<Item = Value> + Send;

    fn midi(self) -> Option<Self::Midi>;
    fn analog(self) -> Option<Self::Analog>;
}

impl<A, B> ValueIter for Either<A, B>
where
    A: ValueIter,
    B: ValueIter,
{
    type Midi = Either<A::Midi, B::Midi>;
    type Analog = Either<A::Analog, B::Analog>;

    fn midi(self) -> Option<Self::Midi> {
        match self {
            Self::Left(val) => val.midi().map(Either::Left),
            Self::Right(val) => val.midi().map(Either::Right),
        }
    }
    fn analog(self) -> Option<Self::Analog> {
        match self {
            Self::Left(val) => val.analog().map(Either::Left),
            Self::Right(val) => val.analog().map(Either::Right),
        }
    }
}

/// Implementation detail since Rust doesn't take associated types into account when checking whether
/// two implementations overlap.
pub trait ValueIterImplHelper<T> {
    type AnyIter: ValueIter + Send;

    fn mk_valueiter(other: T) -> Self::AnyIter;
}

pub enum NoMidi {}
pub enum NoAnalog {}

impl Iterator for NoMidi {
    type Item = MidiEventType;
    fn next(&mut self) -> Option<Self::Item> {
        unreachable!()
    }
}
impl Iterator for NoAnalog {
    type Item = Value;
    fn next(&mut self) -> Option<Self::Item> {
        unreachable!()
    }
}
impl ExactSizeIterator for NoMidi {
    fn len(&self) -> usize {
        unreachable!()
    }
}
impl ExactSizeIterator for NoAnalog {
    fn len(&self) -> usize {
        unreachable!()
    }
}

impl<T: ExactSizeIterator<Item = MidiEventType> + Send> ValueIterImplHelper<T> for MidiEventType {
    type AnyIter = AnyIter<T, NoAnalog>;
    fn mk_valueiter(other: T) -> Self::AnyIter {
        AnyIter(AnyIterInner::Midi(other))
    }
}

impl<T: ExactSizeIterator<Item = Value> + Send> ValueIterImplHelper<T> for Value {
    type AnyIter = AnyIter<NoMidi, T>;

    fn mk_valueiter(other: T) -> Self::AnyIter {
        AnyIter(AnyIterInner::Analog(other))
    }
}

impl<A, B, V> From<V> for AnyIter<A, B>
where
    A: ExactSizeIterator<Item = MidiEventType> + Send,
    B: ExactSizeIterator<Item = Value> + Send,
    V: ExactSizeIterator,
    V::Item: ValueIterImplHelper<V, AnyIter = AnyIter<A, B>>,
{
    fn from(other: V) -> AnyIter<A, B> {
        V::Item::mk_valueiter(other)
    }
}

pub struct AnyIter<A, B>(AnyIterInner<A, B>);

enum AnyIterInner<A, B> {
    Midi(A),
    Analog(B),
}

impl<A, B> ValueIter for AnyIter<A, B>
where
    A: ExactSizeIterator<Item = MidiEventType> + Send,
    B: ExactSizeIterator<Item = Value> + Send,
{
    type Midi = A;
    type Analog = B;

    fn midi(self) -> Option<<Self as ValueIter>::Midi> {
        match self.0 {
            AnyIterInner::Midi(inner) => Some(inner),
            AnyIterInner::Analog(_) => None,
        }
    }

    fn analog(self) -> Option<<Self as ValueIter>::Analog> {
        match self.0 {
            AnyIterInner::Midi(_) => None,
            AnyIterInner::Analog(inner) => Some(inner),
        }
    }
}
