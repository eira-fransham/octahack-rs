use crate::{context::Context, Component};
use std::any::Any;

pub trait ParamStorage {
    type Specifier;

    fn get(&self, spec: Self::Specifier) -> (&dyn Any, &dyn Any);
    fn get_mut(&mut self, spec: Self::Specifier) -> (&mut dyn Any, &mut dyn Any);
}

pub trait Storage {
    type Specifier;
    type Inner;

    fn get(&self, spec: Self::Specifier) -> &Self::Inner;
    fn get_mut(&mut self, spec: Self::Specifier) -> &mut Self::Inner;
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
    type Storage: Storage<Specifier = Self, Inner = T>;
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

impl Param for crate::Value {
    type Extra = crate::rack::InternalParamWire;

    fn access<Ctx>(&self, wire: &Self::Extra, ctx: &Ctx) -> Self
    where
        Ctx: crate::components::anycomponent::AnyContext,
    {
        use crate::{rack::ParamWire, ValueExt};
        use fixed::types::{U0F32, U1F31};
        use staticvec::StaticVec;

        /// Improves precision (and possibly performance, too) by waiting as long as possible to do division.
        /// If we overflow 36 (I believe?) bits total then it crashes, but I believe that it's OK to assume
        /// that doesn't happen.
        fn average_fixed<I>(iter: I) -> U0F32
        where
            I: ExactSizeIterator<Item = U0F32>,
        {
            let len = iter.len() as u32;

            let mut cur = StaticVec::<U0F32, 4>::new();
            let mut acc = U0F32::default();

            for i in iter {
                if let Some(new) = acc.checked_add(i) {
                    acc = new;
                } else {
                    cur.push(acc);
                    acc = i;
                }
            }

            acc / len + cur.into_iter().map(|c| c / len).sum::<U0F32>()
        }

        type UCont = U0F32;

        if let Some(ParamWire { src, value }) = wire {
            let amount = ctx.read_wire(*src);

            fn remap_0_1(val: U1F31) -> U0F32 {
                U0F32::from_bits(val.to_bits())
            }

            fn remap_0_2(val: U0F32) -> U1F31 {
                U1F31::from_bits(val.to_bits())
            }

            let wire_value = remap_0_1(value.to_u());
            let average_output_this_tick: UCont = average_fixed(
                crate::components::PossiblyIter::<crate::Value>::try_iter(amount)
                    .unwrap_or_else(|_| unimplemented!())
                    .map(|o| remap_0_1(o.to_u())),
            );
            let unat = remap_0_1(self.to_u());

            // Weighted average: wire value == max means out is `average_output_this_tick`,
            // wire value == min means out is `unat`, and values between those extremes lerp
            // between the two.
            Self::from_u(remap_0_2(
                (unat * (UCont::max_value() - average_output_this_tick))
                    + wire_value * average_output_this_tick,
            ))
        } else {
            *self
        }
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

    fn get(&self, _: Self::Specifier) -> (&dyn Any, &dyn Any) {
        unreachable!()
    }

    fn get_mut(&mut self, _: Self::Specifier) -> (&mut dyn Any, &mut dyn Any) {
        unreachable!()
    }
}

impl<V> Storage for EmptyStorage<V> {
    type Specifier = !;
    type Inner = V;

    fn get(&self, _: Self::Specifier) -> &Self::Inner {
        unreachable!()
    }

    fn get_mut(&mut self, _: Self::Specifier) -> &mut Self::Inner {
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

                #[derive(Clone)]
                pub enum ValueIter<$($key,)*> {
                    $(
                        $key($key),
                    )*
                }

                impl<__Item, $($key,)*> $crate::components::PossiblyIter<__Item> for ValueIter<$($key,)*>
                where
                    $($key: $crate::components::PossiblyIter<__Item>,)*
                {
                    type Iter = ValueIter<$($key::Iter),*>;

                    fn try_iter(self) -> Result<Self::Iter, Self> {
                        match self {
                            $(
                                Self::$key(val) => val.try_iter()
                                    .map(ValueIter::$key)
                                    .map_err(ValueIter::$key),
                            )*
                        }
                    }
                }

                impl<__Item, $($key,)*> Iterator for ValueIter<$($key,)*>
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

                impl<__Item, $($key,)*> ExactSizeIterator for ValueIter<$($key,)*>
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
                    type Iter = ValueIter<$(
                        <$value as
                            $crate::components::ValueIterImplHelper<<C as $crate::GetOutput<$key>>::Iter>>::AnyIter,
                    )*>;

                    fn get_output<Ctx>(self, comp: &C, ctx: &Ctx) -> Self::Iter
                    where
                        Ctx: $crate::context::Context<C>,
                    {
                        match self {
                            $(
                                Self::$key => ValueIter::$key(
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

                impl $crate::RuntimeSpecifier for Specifier {
                    const VALUES: &'static [Self] = &[ $( Specifier::$key ),* ];
                    const TYPES: &'static [$crate::ValueType] = &[
                        $( (stringify!($key), $crate::ValueType::mono()).1 ),*
                    ];

                    #[allow(irrefutable_let_patterns, unused_assignments)]
                    fn id(&self) -> $crate::SpecId {
                        let mut i = 0;
                        $(
                            if let Specifier::$key = self { return i; }
                            i += 1;
                        )*

                        unreachable!()
                    }

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

                impl<V> $crate::params::Storage for Storage<V> {
                    type Specifier = Specifier;
                    type Inner = V;

                    fn get(&self, spec: Self::Specifier) -> &Self::Inner {
                        match spec {
                            $(
                                Specifier::$key => &self.$key,
                            )*
                        }
                    }

                    fn get_mut(&mut self, spec: Self::Specifier) -> &mut Self::Inner {
                        match spec {
                            $(
                                Specifier::$key => &mut self.$key,
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

                    fn get(&self, spec: Self::Specifier) -> (&dyn std::any::Any, &dyn std::any::Any) {
                        match spec {
                            $(
                                Specifier::$key => {
                                    (&self.params.$key, &self.extra.$key)
                                },
                            )*
                        }
                    }

                    fn get_mut(&mut self, spec: Self::Specifier) -> (&mut dyn std::any::Any, &mut dyn std::any::Any) {
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
                A: Value::saturating_from_num(0),
                B: MidiValue::ChannelPressure(0),
            }
        }
    }
}
