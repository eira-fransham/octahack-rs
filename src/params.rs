use crate::{
    components::PossiblyIter,
    context::Context,
    rack::{marker, InternalWire, ParamValue, ParamWire, Wire},
    AnyInputSpec, AnyOutputSpec, Component, RefRuntimeSpecifier, RuntimeSpecifier, Value,
};
use std::{
    any::Any,
    fmt, iter,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub trait DisplayParam: Key {
    type Display: fmt::Display;

    fn display(val: Self::Value) -> Self::Display;
}

pub trait DisplayParamValue: HasParamStorage {
    type Display: fmt::Display;

    fn display(&self, val: &dyn Any) -> Self::Display;
}

pub trait ParamStorage {
    type Specifier;

    fn get(&self, spec: &Self::Specifier) -> (&dyn Any, &dyn Any);
    fn get_mut(&mut self, spec: &Self::Specifier) -> (&mut dyn Any, &mut dyn Any);
}

pub trait Storage {
    type Specifier;
    type Inner;

    fn get(&self, spec: &Self::Specifier) -> &Self::Inner;
}

impl<T> Storage for T
where
    T: Deref,
    T::Target: Storage,
{
    type Specifier = <T::Target as Storage>::Specifier;
    type Inner = <T::Target as Storage>::Inner;

    fn get(&self, spec: &Self::Specifier) -> &Self::Inner {
        <T::Target as Storage>::get(&**self, spec)
    }
}

pub trait StorageMut: Storage {
    fn set(&mut self, spec: &Self::Specifier, val: Self::Inner);
}

impl<T> StorageMut for T
where
    T: DerefMut,
    T::Target: StorageMut,
{
    fn set(&mut self, spec: &Self::Specifier, val: Self::Inner) {
        <T::Target as StorageMut>::set(&mut **self, spec, val)
    }
}

pub enum EitherStorage<A, B> {
    Left(A),
    Right(B),
}

impl<A, B> Storage for EitherStorage<A, B>
where
    A: Storage,
    B: Storage<Specifier = A::Specifier, Inner = A::Inner>,
{
    type Specifier = A::Specifier;
    type Inner = A::Inner;

    fn get(&self, spec: &Self::Specifier) -> &Self::Inner {
        match self {
            Self::Left(val) => val.get(spec),
            Self::Right(val) => val.get(spec),
        }
    }
}

impl<A, B> ParamStorage for EitherStorage<A, B>
where
    A: ParamStorage,
    B: ParamStorage<Specifier = A::Specifier>,
{
    type Specifier = A::Specifier;

    fn get(&self, spec: &Self::Specifier) -> (&dyn Any, &dyn Any) {
        match self {
            Self::Left(val) => val.get(spec),
            Self::Right(val) => val.get(spec),
        }
    }

    fn get_mut(&mut self, spec: &Self::Specifier) -> (&mut dyn Any, &mut dyn Any) {
        match self {
            Self::Left(val) => val.get_mut(spec),
            Self::Right(val) => val.get_mut(spec),
        }
    }
}

impl<A, B> StorageMut for EitherStorage<A, B>
where
    A: StorageMut,
    B: StorageMut<Specifier = A::Specifier, Inner = A::Inner>,
{
    fn set(&mut self, spec: &Self::Specifier, val: Self::Inner) {
        match self {
            Self::Left(inner) => inner.set(spec, val),
            Self::Right(inner) => inner.set(spec, val),
        }
    }
}

pub type OutputIterForComponent<C> = <<C as Component>::OutputSpecifier as Output<C>>::Iter;

pub trait Output<C>
where
    C: Component,
{
    type Iter;

    fn get_output<Ctx>(self, comp: &C, ctx: &Ctx) -> Self::Iter
    where
        Ctx: Context<C>;
}

pub trait HasStorage<T>: Sized {
    type Storage: StorageMut<Specifier = Self, Inner = T>;
}

pub struct AnyOptStorage<S, T> {
    inner: Vec<Option<T>>,
    _marker: PhantomData<S>,
}

impl<S, T> Default for AnyOptStorage<S, T> {
    fn default() -> Self {
        AnyOptStorage {
            inner: vec![],
            _marker: PhantomData,
        }
    }
}

impl<'a, S: 'a, T: 'a> IntoIterator for &'a AnyOptStorage<S, T>
where
    S: RuntimeSpecifier,
{
    type Item = (S, &'a Option<T>);
    type IntoIter = impl Iterator<Item = Self::Item> + Clone + 'a;

    fn into_iter(self) -> Self::IntoIter {
        self.inner
            .iter()
            .enumerate()
            .filter(|(_, x)| x.is_some())
            .map(|(i, x)| (S::from_id(i), x))
    }
}

impl<S, T> Storage for AnyOptStorage<S, T>
where
    S: RefRuntimeSpecifier,
{
    type Specifier = S;
    type Inner = Option<T>;

    fn get(&self, spec: &Self::Specifier) -> &Self::Inner {
        self.inner.get(spec.id()).unwrap_or(&None)
    }
}

impl<S, T> StorageMut for AnyOptStorage<S, T>
where
    S: RefRuntimeSpecifier,
{
    fn set(&mut self, spec: &Self::Specifier, val: Self::Inner) {
        let diff = (1 + spec.id()).saturating_sub(self.inner.len());

        if diff > 0 {
            self.inner.extend(iter::repeat_with(|| None).take(diff));
        }

        self.inner[spec.id()] = val;
    }
}

impl<T> HasStorage<Option<T>> for AnyOutputSpec {
    type Storage = AnyOptStorage<Self, T>;
}

impl<T> HasStorage<Option<T>> for AnyInputSpec {
    type Storage = AnyOptStorage<Self, T>;
}

pub trait HasParamStorage: Sized {
    type Storage: ParamStorage<Specifier = Self>;
}

pub trait Param {
    type Extra: Default;

    fn access<Ctx>(&self, storage: &Self::Extra, ctx: &Ctx) -> Self
    where
        Ctx: crate::components::anycomponent::AnyContext;
}

impl Param for crate::MidiValue {
    type Extra = ();

    fn access<Ctx>(&self, _: &(), _: &Ctx) -> Self
    where
        Ctx: crate::components::anycomponent::AnyContext,
    {
        *self
    }
}

// TODO: Can you wire this? How would that work?
impl<Kind> Param for Option<crate::context::FileId<Kind>> {
    type Extra = ();

    fn access<Ctx>(&self, _: &(), _: &Ctx) -> Self
    where
        Ctx: crate::components::anycomponent::AnyContext,
    {
        *self
    }
}

fn access_value<Ctx>(val: Value, wire: Option<&ParamWire>, ctx: &Ctx) -> Value
where
    Ctx: crate::components::anycomponent::AnyContext,
{
    if let Some(ParamWire { src, cv }) = wire {
        let amount = ctx.read_wire(*src);

        let average_output_this_tick: Value = amount
            .map(|amount| {
                let mut iter =
                    PossiblyIter::<Value>::try_iter(amount).unwrap_or_else(|_| unimplemented!());
                let len = iter.len();
                iter.sum::<Value>() / len as f64
            })
            .unwrap_or_default();

        let cv = access_value(cv.natural_value, cv.wire.as_ref().map(|w| &**w), ctx);

        val + cv * average_output_this_tick
    } else {
        val
    }
}

impl Param for crate::Value {
    type Extra = crate::rack::InternalParamWire;

    fn access<Ctx>(&self, wire: &Self::Extra, ctx: &Ctx) -> Self
    where
        Ctx: crate::components::anycomponent::AnyContext,
    {
        access_value(*self, wire.as_ref(), ctx)
    }
}

impl HasParamStorage for ! {
    type Storage = EmptyParamStorage;
}

impl<T: 'static> HasStorage<T> for ! {
    type Storage = EmptyStorage<T>;
}

pub struct EmptyStorage<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> Default for EmptyStorage<T> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[derive(Default)]
pub struct EmptyParamStorage {
    _noconstruct: (),
}

impl ParamStorage for EmptyParamStorage {
    type Specifier = !;

    fn get(&self, _: &Self::Specifier) -> (&dyn Any, &dyn Any) {
        unreachable!()
    }

    fn get_mut(&mut self, _: &Self::Specifier) -> (&mut dyn Any, &mut dyn Any) {
        unreachable!()
    }
}

impl<V> Storage for EmptyStorage<V> {
    type Specifier = !;
    type Inner = V;

    fn get(&self, _: &Self::Specifier) -> &Self::Inner {
        unreachable!()
    }
}

impl<V> StorageMut for EmptyStorage<V> {
    fn set(&mut self, _: &Self::Specifier, val: Self::Inner) {
        unreachable!()
    }
}

pub trait ParamStorageGet<V>
where
    V: Key,
    V::Value: Param,
{
    fn get(&self) -> (&V::Value, &<V::Value as Param>::Extra);
    fn get_mut(&mut self) -> (&mut V::Value, &mut <V::Value as Param>::Extra);
}

pub trait StorageGet<V>: Storage {
    fn get(&self) -> &<Self as Storage>::Inner;
    fn get_mut(&mut self) -> &mut <Self as Storage>::Inner;
}

pub trait Key {
    type Value;
}

#[macro_export]
macro_rules! specs {
    ($( $v:vis mod $modname:ident { $($key:ident : $value:ty),* } )*) => {
        $(
            $v mod $modname {
                #[derive(Copy, Clone, PartialEq, Eq)]
                pub enum Specifier {
                    $( $key, )*
                }

                impl $crate::params::DisplayParamValue for Specifier
                where
                    $( $key: $crate::params::DisplayParam, )*
                {
                    type Display = OneOf<
                        $( <$key as $crate::params::DisplayParam>::Display, )*
                    >;

                    fn display(&self, val: &dyn std::any::Any) -> Self::Display {
                        match self {
                            $(
                                Specifier::$key => {
                                    OneOf::$key(
                                        <$key as $crate::params::DisplayParam>::display(
                                            val.downcast_ref::<
                                                <$key as $crate::params::Key>::Value
                                            >().unwrap().clone()
                                        )
                                    )
                                }
                            )*
                        }
                    }
                }

                #[derive(Clone)]
                pub enum OneOf<$($key,)*> {
                    $(
                        $key($key),
                    )*
                }

                impl<$($key,)*> std::fmt::Display for OneOf<$($key,)*>
                where
                    $($key: std::fmt::Display,)*
                {
                    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        match self {
                            $(
                                Self::$key(val) => val.fmt(f),
                            )*
                        }
                    }
                }

                impl<__Item, $($key,)*> $crate::components::PossiblyIter<__Item> for OneOf<$($key,)*>
                where
                    $($key: $crate::components::PossiblyIter<__Item>,)*
                {
                    type Iter = OneOf<$($key::Iter),*>;

                    fn try_iter(self) -> Result<Self::Iter, Self> {
                        match self {
                            $(
                                Self::$key(val) => val.try_iter()
                                    .map(OneOf::$key)
                                    .map_err(OneOf::$key),
                            )*
                        }
                    }
                }

                impl<__Item, $($key,)*> Iterator for OneOf<$($key,)*>
                where
                    $( $key: Iterator<Item = __Item>, )*
                {
                    type Item = __Item;

                    fn next(&mut self) -> Option<Self::Item> {
                        match self {
                            $(
                                Self::$key(val) => val.next(),
                            )*
                        }
                    }
                }

                impl<__Item, $($key,)*> ExactSizeIterator for OneOf<$($key,)*>
                where
                    $( $key: ExactSizeIterator<Item = __Item>, )*
                {
                    fn len(&self) -> usize {
                        match self {
                            $(
                                Self::$key(val) => val.len(),
                            )*
                        }
                    }
                }

                impl<C> $crate::params::Output<C> for Specifier
                where
                    C: $crate::Component,
                    $(
                        C: $crate::GetOutput<$key>,
                        $value: $crate::components::ValueIterImplHelper<<C as $crate::GetOutput<$key>>::Iter>,
                        <C as $crate::GetOutput<$key>>::Iter: Into<
                            <$value as
                                $crate::components::ValueIterImplHelper<
                                    <C as $crate::GetOutput<$key>>::Iter
                                >
                            >::AnyIter
                        >,
                    )*
                {
                    type Iter = OneOf<$(
                        <$value as
                            $crate::components::ValueIterImplHelper<
                                <C as $crate::GetOutput<$key>>::Iter
                            >
                        >::AnyIter,
                    )*>;

                    fn get_output<Ctx>(self, comp: &C, ctx: &Ctx) -> Self::Iter
                    where
                        Ctx: $crate::context::Context<C>,
                    {
                        match self {
                            $(
                                Self::$key => OneOf::$key(
                                    <C as $crate::GetOutput<$key>>::output(comp, ctx).into()
                                ),
                            )*
                        }
                    }
                }

                impl $crate::params::HasParamStorage for Specifier {
                    type Storage = ParamsWithExtra;
                }

                impl<T: 'static> $crate::params::HasStorage<T> for Specifier {
                    type Storage = Storage<T>;
                }

                impl std::fmt::Display for Specifier {
                    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        match self {
                            $(
                                Specifier::$key => write!(f, stringify!($key)),
                            )*
                        }
                    }
                }

                impl $crate::RefRuntimeSpecifier for Specifier {
                    #[allow(irrefutable_let_patterns, unused_assignments)]
                    fn id(&self) -> $crate::SpecId {
                        let mut i = 0;
                        $(
                            if let Specifier::$key = self { return i; }
                            i += 1;
                        )*

                        unreachable!()
                    }

                    fn value_type(&self) -> $crate::ValueType {
                        [
                            $( (stringify!($key), $crate::ValueType::mono()).1 ),*
                        ][self.id()]
                    }
                }

                impl $crate::RuntimeSpecifier for Specifier {
                    #[allow(irrefutable_let_patterns, unused_assignments)]
                    fn from_id(id: $crate::SpecId) -> Self {
                        let mut i = 0;
                        $(
                            if i == id { return Specifier::$key; }
                            i += 1;
                        )*

                        unreachable!()
                    }
                }

                impl $crate::components::EnumerateValues for Specifier {
                    type Iter = std::iter::Copied<std::slice::Iter<'static, &'static Self>>;

                    fn values() -> Self::Iter {
                        [ $( &Specifier::$key ),* ].iter().copied()
                    }
                }

                $(
                    pub enum $key {}
                    impl $crate::params::Key for $key {
                        type Value = $value;
                    }
                )*

                #[derive(Default)]
                #[allow(non_snake_case)]
                pub struct Storage<V> {
                    $(
                        $key : V,
                    )*
                }

                impl<'a, V: 'a> std::iter::IntoIterator for &'a Storage<V> {
                    type Item = (Specifier, &'a V);
                    type IntoIter = std::iter::Zip<
                        std::iter::Copied<<Specifier as $crate::components::EnumerateValues>::Iter>,
                        $crate::array_iterator::ArrayIterator<
                            &'a V,
                            [&'a V; $( (stringify!($key), 1).1 +)* 0]
                        >,
                    >;

                    fn into_iter(self) -> Self::IntoIter {
                        use $crate::{components::EnumerateValues, array_iterator::ArrayIterator};

                        Specifier::values()
                            .copied()
                            .zip(
                                ArrayIterator::new([$( &self.$key, )*])
                            )
                    }
                }

                impl<V> $crate::params::Storage for Storage<V> {
                    type Specifier = Specifier;
                    type Inner = V;

                    fn get(&self, spec: &Self::Specifier) -> &Self::Inner {
                        match spec {
                            $(
                                Specifier::$key => &self.$key,
                            )*
                        }
                    }
                }

                impl<V> $crate::params::StorageMut for Storage<V> {
                    fn set(&mut self, spec: &Self::Specifier, val: Self::Inner) {
                        match spec {
                            $(
                                Specifier::$key => self.$key = val,
                            )*
                        }
                    }
                }

                #[allow(non_snake_case)]
                pub struct ParamsWithExtra {
                    params: Params,
                    extra: Extra,
                }

                impl Default for ParamsWithExtra where Params: Default, {
                    fn default() -> Self {
                        Self {
                            params: Default::default(),
                            extra: Default::default(),
                        }
                    }
                }

                #[allow(non_snake_case)]
                #[derive(Default)]
                struct Extra {
                    $(
                        $key : <$value as $crate::params::Param>::Extra,
                    )*
                }

                #[allow(non_snake_case)]
                pub struct Params {
                    $(
                        pub $key : $value,
                    )*
                }

                $(
                    impl $crate::params::ParamStorageGet<$key> for ParamsWithExtra {
                        fn get(&self) -> (
                            &<$key as $crate::params::Key>::Value,
                            &<<$key as $crate::params::Key>::Value as $crate::params::Param>::Extra
                        ) {
                            (&self.params.$key, &self.extra.$key)
                        }

                        fn get_mut(&mut self) -> (
                            &mut <$key as $crate::params::Key>::Value,
                            &mut <<$key as $crate::params::Key>::Value as $crate::params::Param>::Extra
                        ) {
                            (&mut self.params.$key, &mut self.extra.$key)
                        }
                    }

                    impl<T> $crate::params::StorageGet<$key> for Storage<T> {
                        fn get(&self) -> &T {
                            &self.$key
                        }

                        fn get_mut(&mut self) -> &mut T {
                            &mut self.$key
                        }
                    }
                )*

                impl $crate::params::ParamStorage for ParamsWithExtra {
                    type Specifier = Specifier;

                    fn get(&self, spec: &Self::Specifier) -> (&dyn std::any::Any, &dyn std::any::Any) {
                        match spec {
                            $(
                                Specifier::$key => {
                                    (&self.params.$key, &self.extra.$key)
                                },
                            )*
                        }
                    }

                    fn get_mut(&mut self, spec: &Self::Specifier) -> (&mut dyn std::any::Any, &mut dyn std::any::Any) {
                        match spec {
                            $(
                                Specifier::$key => {
                                    (&mut self.params.$key, &mut self.extra.$key)
                                },
                            )*
                        }
                    }
                }
            }
        )*
    };
}

#[cfg(test)]
mod tests {
    use crate::*;

    specs! {
        mod foo {
            A: crate::Value,
            B: crate::MidiValue
        }
    }

    impl Default for foo::Params {
        fn default() -> Self {
            Self {
                A: 0.,
                B: MidiValue::ChannelPressure(0),
            }
        }
    }
}
