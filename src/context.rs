use crate::{
    components::{anycomponent::AnyContext, PossiblyIter},
    params::{HasParamStorage, HasStorage, Key, ParamStorageGet, StorageGet},
    rack::{InternalParamWire, InternalWire, Lerp, ParamWire},
    Component, Value,
};
use nom_midi::MidiEventType;
use std::{convert::TryInto, marker::PhantomData, time::Duration};

pub struct FileId<Kind> {
    index: usize,
    _marker: PhantomData<Kind>,
}

impl<Kind> Clone for FileId<Kind> {
    fn clone(&self) -> Self {
        FileId {
            index: self.index,
            _marker: PhantomData,
        }
    }
}

pub trait File<Kind> {
    type Samples: Iterator<Item = Kind>;

    fn at(&self, dur: Duration) -> Self::Samples;
    fn between(&self, last: Duration, dur: Duration) -> Self::Samples;
}

pub trait FileAccess<Kind> {
    type ReadFile: File<Kind>;

    // Will always read the file from the start
    fn read(&self, id: FileId<Kind>) -> Self::ReadFile;
}

pub trait GetGlobalInput<Spec> {
    type Iter: PossiblyIter<Value> + PossiblyIter<MidiEventType>;

    // `None` means that this input is not wired
    fn input(&self, spec: Spec) -> Option<Self::Iter>;
}

pub trait GetInput<Spec> {
    type Iter: PossiblyIter<Value> + PossiblyIter<MidiEventType>;

    // `None` means that this input is not wired
    fn input<T: Key>(&self) -> Option<<Self::Iter as PossiblyIter<T::Value>>::Iter>
    where
        Spec: HasStorage<InternalWire>,
        Spec::Storage: StorageGet<T>,
        Self::Iter: PossiblyIter<T::Value>;
}

pub trait GetParam<Spec> {
    fn param<T: Key>(&self) -> T::Value
    where
        Spec: HasParamStorage<InternalParamWire>,
        Spec::Storage: ParamStorageGet<T, Extra = InternalParamWire, Output = T::Value>,
        T::Value: Clone + crate::rack::Lerp;
}

pub trait ContextMeta {
    /// Samples per second
    fn sample_rate(&self) -> u32;
}

pub trait Context<C: Component>:
    GetInput<C::InputSpecifier> + GetParam<C::ParamSpecifier> + ContextMeta
{
}

impl<T, C> Context<C> for T
where
    C: Component,
    T: GetInput<C::InputSpecifier> + GetParam<C::ParamSpecifier> + ContextMeta,
{
}

pub struct ContextForComponent<'a, Ctx, C> {
    ctx: &'a Ctx,
    _marker: PhantomData<C>,
}

impl<'a, Ctx, C> ContextForComponent<'a, Ctx, C> {
    pub fn new(ctx: &'a Ctx) -> Self {
        Self {
            ctx,
            _marker: PhantomData,
        }
    }
}

impl<'a, Ctx, C> ContextMeta for ContextForComponent<'a, Ctx, C>
where
    Ctx: ContextMeta,
{
    fn sample_rate(&self) -> u32 {
        self.ctx.sample_rate()
    }
}

impl<'a, Ctx, C> GetInput<C::InputSpecifier> for ContextForComponent<'a, Ctx, C>
where
    C: Component,
    C::InputSpecifier: HasStorage<InternalWire>,
    Ctx: AnyContext,
    for<'any> &'any Ctx::InputStorage:
        TryInto<&'any <C::InputSpecifier as HasStorage<InternalWire>>::Storage>,
{
    type Iter = Ctx::Iter;

    fn input<T: Key>(&self) -> Option<<Self::Iter as PossiblyIter<T::Value>>::Iter>
    where
        C::InputSpecifier: HasStorage<InternalWire>,
        <C::InputSpecifier as HasStorage<InternalWire>>::Storage: StorageGet<T>,
        Self::Iter: PossiblyIter<T::Value>,
    {
        self.ctx
            .inputs()
            .try_into()
            .unwrap_or_else(|_| unreachable!())
            .get()
            .map(|wire| {
                self.ctx
                    .read_wire(wire)
                    .try_iter()
                    .unwrap_or_else(|_| unreachable!())
            })
    }
}

impl<'a, Ctx, C> GetParam<C::ParamSpecifier> for ContextForComponent<'a, Ctx, C>
where
    C: Component,
    C::ParamSpecifier: HasParamStorage<InternalParamWire>,
    Ctx: AnyContext,
    Ctx::Iter: PossiblyIter<Value>,
    for<'any> &'any Ctx::ParamStorage:
        TryInto<&'any <C::ParamSpecifier as HasParamStorage<InternalParamWire>>::Storage>,
{
    fn param<T: Key>(&self) -> T::Value
    where
        C::ParamSpecifier: HasParamStorage<InternalParamWire>,
        <C::ParamSpecifier as HasParamStorage<InternalParamWire>>::Storage:
            ParamStorageGet<T, Extra = InternalParamWire, Output = T::Value>,
        T::Value: Clone + crate::rack::Lerp,
    {
        let (nat_val, wire) = self
            .ctx
            .params()
            .try_into()
            .unwrap_or_else(|_| unreachable!())
            .get();

        wire.as_ref()
            .map(|ParamWire { src, value }| {
                nat_val
                    .clone()
                    .lerp(*value, self.ctx.read_wire(*src).try_iter().ok())
            })
            .unwrap_or(nat_val.clone())
    }
}
