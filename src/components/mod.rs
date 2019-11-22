pub mod anycomponent;

pub use anycomponent::{AnyComponent, AnyInputSpec, AnyOutputSpec, AnyParamSpec, Types};

use crate::{params::Specifier, GetInput, GetParam, Value, ValueType};
use itertools::Either;
use nom_midi::MidiEventType;

// TODO: This can probably be `u8`
pub type SpecId = usize;

pub trait RuntimeSpecifier: Sized + Clone + 'static {
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

impl RuntimeSpecifier for ! {
    const VALUES: &'static [Self] = &[];
    const TYPES: &'static [ValueType] = &[];

    fn id(&self) -> SpecId {
        unreachable!()
    }
}

pub trait Component: Sized {
    type InputSpecifier: Specifier;
    type OutputSpecifier: Specifier;
    type ParamSpecifier: Specifier;
    // TODO: Use GATs to allow adapators to be used internally.
    type OutputIter: ValueIter + Send;
}

pub trait Update<Ctx> {
    fn update(&self, _ctx: Ctx) -> Self;
}

pub trait GetOutput<Ctx, RuntimeSpecifier>: Update<Ctx> + Component {
    fn output(&self, ctx: Ctx) -> Self::OutputIter;
}

// TODO: Support MIDI inputs

pub trait ValueIter: PossiblyIter<MidiEventType> + PossiblyIter<Value> {
    type Midi: ExactSizeIterator<Item = MidiEventType> + Send;
    type Analog: ExactSizeIterator<Item = Value> + Send;

    fn midi(self) -> Option<Self::Midi>;
    fn analog(self) -> Option<Self::Analog>;
}

impl<T> ValueIter for T
where
    T: PossiblyIter<MidiEventType> + PossiblyIter<Value>,
{
    type Midi = <T as PossiblyIter<MidiEventType>>::Iter;
    type Analog = <T as PossiblyIter<Value>>::Iter;

    fn midi(self) -> Option<Self::Midi> {
        <Self as PossiblyIter<MidiEventType>>::try_iter(self).ok()
    }

    fn analog(self) -> Option<Self::Analog> {
        <Self as PossiblyIter<Value>>::try_iter(self).ok()
    }
}

pub trait PossiblyIter<T>: Sized {
    type Iter: ExactSizeIterator<Item = T> + Send;

    fn try_iter(self) -> Result<Self::Iter, Self>;
}

impl<A, B, T> PossiblyIter<T> for Either<A, B>
where
    A: PossiblyIter<T>,
    B: PossiblyIter<T>,
{
    type Iter = Either<A::Iter, B::Iter>;

    fn try_iter(self) -> Result<Self::Iter, Self> {
        match self {
            Self::Left(val) => val.try_iter().map(Either::Left).map_err(Either::Left),
            Self::Right(val) => val.try_iter().map(Either::Right).map_err(Either::Right),
        }
    }
}

/// Implementation detail since Rust doesn't take associated types into account when checking whether
/// two implementations overlap.
pub trait ValueIterImplHelper<T> {
    type AnyIter: ValueIter + Send;

    fn mk_valueiter(other: T) -> Self::AnyIter;
}

pub struct NoIter<T> {
    _noconstruct: !,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Iterator for NoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        unreachable!()
    }
}
impl<T> ExactSizeIterator for NoIter<T> {
    fn len(&self) -> usize {
        unreachable!()
    }
}

impl<T: ExactSizeIterator<Item = MidiEventType> + Send> ValueIterImplHelper<T> for MidiEventType {
    type AnyIter = AnyIter<T, NoIter<Value>>;
    fn mk_valueiter(other: T) -> Self::AnyIter {
        AnyIter(AnyIterInner::Midi(other))
    }
}

impl<T: ExactSizeIterator<Item = Value> + Send> ValueIterImplHelper<T> for Value {
    type AnyIter = AnyIter<NoIter<MidiEventType>, T>;

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

impl<A, B> PossiblyIter<MidiEventType> for AnyIter<A, B>
where
    A: ExactSizeIterator<Item = MidiEventType> + Send,
{
    type Iter = A;

    fn try_iter(self) -> Result<Self::Iter, Self> {
        match self.0 {
            AnyIterInner::Midi(inner) => Ok(inner),
            this @ AnyIterInner::Analog(_) => Err(AnyIter(this)),
        }
    }
}

impl<A, B> PossiblyIter<Value> for AnyIter<A, B>
where
    B: ExactSizeIterator<Item = Value> + Send,
{
    type Iter = B;

    fn try_iter(self) -> Result<Self::Iter, Self> {
        match self.0 {
            AnyIterInner::Analog(inner) => Ok(inner),
            this @ AnyIterInner::Midi(_) => Err(AnyIter(this)),
        }
    }
}
