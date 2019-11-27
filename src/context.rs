use crate::{
    components::anycomponent::AnyContext,
    params::{HasParamStorage, HasStorage, Key, ParamStorageGet, StorageGet},
    rack::{InternalParamWire, InternalWire, Lerp, ParamWire},
    Component, Value, ValueIter,
};
use std::{convert::TryInto, marker::PhantomData};

pub trait ContextMeta {
    /// Samples per second
    fn samples(&self) -> usize;
}

pub struct FileId<Kind> {
    index: usize,
    _marker: PhantomData<Kind>,
}

pub trait FileAccess<Kind> {
    type ReadFile;

    // Will always read the file from the start
    fn read(&self, id: FileId<Kind>) -> Option<Self::ReadFile>;
}

pub trait GetGlobalInput<Spec> {
    type Iter: ValueIter + Send;

    // `None` means that this input is not wired
    fn input(&self, spec: Spec) -> Option<Self::Iter>;
}

pub trait GetInput<Spec> {
    type Iter: ValueIter + Send;

    // `None` means that this input is not wired
    fn input<T: Key>(&self) -> Option<Self::Iter>
    where
        Spec: HasStorage<InternalWire>,
        Spec::Storage: StorageGet<T>;
}

pub trait GetParam<Spec: crate::params::Specifier> {
    fn param<T: Key>(&self) -> T::Value
    where
        Spec: HasParamStorage<InternalParamWire>,
        Spec::Storage: ParamStorageGet<T, Extra = InternalParamWire, Output = T::Value>,
        T::Value: Clone + crate::rack::Lerp;
}

pub struct Context<'a, Ctx, C> {
    ctx: &'a Ctx,
    _marker: PhantomData<C>,
}

impl<'a, Ctx, C> Context<'a, Ctx, C> {
    pub fn new(ctx: &'a Ctx) -> Self {
        Self {
            ctx,
            _marker: PhantomData,
        }
    }
}

impl<'a, Ctx, C> GetInput<C::InputSpecifier> for Context<'a, Ctx, C>
where
    C: Component,
    C::InputSpecifier: HasStorage<InternalWire>,
    Ctx: AnyContext,
    for<'any> &'any Ctx::InputStorage:
        TryInto<&'any <C::InputSpecifier as HasStorage<InternalWire>>::Storage>,
{
    type Iter = Ctx::Iter;

    // `None` means that this input is not wired
    fn input<T: Key>(&self) -> Option<Self::Iter>
    where
        C::InputSpecifier: HasStorage<InternalWire>,
        <C::InputSpecifier as HasStorage<InternalWire>>::Storage: StorageGet<T>,
    {
        self.ctx
            .inputs()
            .try_into()
            .unwrap_or_else(|_| unreachable!())
            .get()
            .map(|wire| self.ctx.read_wire(wire))
    }
}

impl<'a, Ctx, C> GetParam<C::ParamSpecifier> for Context<'a, Ctx, C>
where
    C: Component,
    C::ParamSpecifier: crate::params::Specifier + HasParamStorage<InternalParamWire>,
    Ctx: AnyContext,
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
                Lerp::lerp(nat_val, value, self.ctx.read_wire(*src).analog())
            })
            .unwrap_or(nat_val.clone())
    }
}
