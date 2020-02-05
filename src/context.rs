use crate::{
    components::{anycomponent::AnyContext, PossiblyIter},
    params::{HasParamStorage, HasStorage, Key, Param, ParamStorageGet, StorageGet},
    rack::InternalWire,
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

impl<Kind> Copy for FileId<Kind> {}

pub trait File<Kind> {
    type SamplesAt: ExactSizeIterator<Item = Kind>;
    type SamplesBetween: ExactSizeIterator<Item = Self::SamplesAt>;

    fn at(&self, dur: Duration) -> Self::SamplesAt;
    fn between(&self, last: Duration, dur: Duration) -> Self::SamplesBetween;
}

impl<Kind> File<Kind> for ! {
    type SamplesAt = std::iter::Empty<Kind>;
    type SamplesBetween = std::iter::Empty<Self::SamplesAt>;

    fn at(&self, dur: Duration) -> Self::SamplesAt {
        todo!()
    }

    fn between(&self, last: Duration, dur: Duration) -> Self::SamplesBetween {
        todo!()
    }
}

pub trait FileAccess<Kind> {
    type ReadFile: File<Kind>;

    // Will always read the file from the start
    fn read(&self, id: FileId<Kind>) -> Self::ReadFile;
}

impl<T, Kind> FileAccess<Kind> for T {
    type ReadFile = !;

    // Will always read the file from the start
    fn read(&self, id: FileId<Kind>) -> Self::ReadFile {
        todo!()
    }
}

pub trait GetFunctionParam {
    type InputSpec;
    type Iter: PossiblyIter<Value> + PossiblyIter<MidiEventType>;

    // `None` means that this input is not wired
    fn input(&self, spec: Self::InputSpec) -> Option<Self::Iter>;
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

pub trait GetParam<Spec: HasParamStorage> {
    fn param<T: Key>(&self) -> T::Value
    where
        Spec::Storage: ParamStorageGet<T>,
        T::Value: Param;
}

pub trait ContextMeta {
    /// Samples per second
    fn sample_rate(&self) -> u32;
}

pub trait ContextMetaExt: ContextMeta {
    fn sample_duration(&self) -> Duration;
}

impl<T> ContextMetaExt for T
where
    T: ContextMeta,
{
    fn sample_duration(&self) -> Duration {
        Duration::from_secs(1) / self.sample_rate()
    }
}

pub trait Context<C: Component>:
    // TODO: Can we bound in such a way that will be `FileAccess` for any `T` for which we have a `FileId<T>`
    //       parameter?
    GetInput<C::InputSpecifier> + GetParam<C::ParamSpecifier> + ContextMeta + FileAccess<Value>
{
}

impl<T, C> Context<C> for T
where
    C: Component,
    T: GetInput<C::InputSpecifier> + GetParam<C::ParamSpecifier> + ContextMeta + FileAccess<Value>,
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
            .and_then(|wire| {
                Some(
                    self.ctx
                        .read_wire(wire)?
                        .try_iter()
                        .unwrap_or_else(|_| unreachable!()),
                )
            })
    }
}

impl<'a, Ctx, C> GetParam<C::ParamSpecifier> for ContextForComponent<'a, Ctx, C>
where
    C: Component,
    Ctx: AnyContext,
    Ctx::Iter: PossiblyIter<Value>,
    for<'any> &'any Ctx::ParamStorage:
        TryInto<&'any <C::ParamSpecifier as HasParamStorage>::Storage>,
{
    fn param<T: Key>(&self) -> T::Value
    where
        <C::ParamSpecifier as HasParamStorage>::Storage: ParamStorageGet<T>,
        T::Value: Param,
    {
        let (nat_val, wire) = self
            .ctx
            .params()
            .try_into()
            .unwrap_or_else(|_| unreachable!())
            .get();

        nat_val.access(wire, self.ctx)
    }
}
