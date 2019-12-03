use crate::{context::Context, Component};

pub trait ParamStorage<'a> {
    type Ref;
    type RefMut;
    type Specifier;
    type Extra;

    fn get(&'a self, spec: Self::Specifier) -> (Self::Ref, &'a Self::Extra);
    fn get_mut(&'a mut self, spec: Self::Specifier) -> (Self::RefMut, &'a mut Self::Extra);
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

pub trait HasParamStorage<T>: Sized {
    type Storage: for<'a> ParamStorage<'a, Specifier = Self, Extra = T>;
}

impl<T: 'static> HasParamStorage<T> for ! {
    type Storage = EmptyStorage<T>;
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

impl<'a, T: 'a> ParamStorage<'a> for EmptyStorage<T> {
    type Ref = !;
    type RefMut = !;
    type Specifier = !;
    type Extra = T;

    fn get(&self, _: Self::Specifier) -> (Self::Ref, &Self::Extra) {
        unreachable!()
    }

    fn get_mut(&mut self, _: Self::Specifier) -> (Self::RefMut, &mut Self::Extra) {
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

pub trait ParamStorageGet<V> {
    type Output;
    type Extra;

    fn get(&self) -> (&Self::Output, &Self::Extra);
    fn get_mut(&mut self) -> (&mut Self::Output, &mut Self::Extra);
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
    ($( $v:vis mod $modname:ident { $($key:ident : $value:ident),* } )*) => {
        $(
            $v mod $modname {
                use $crate::derive_more::TryInto;

                #[derive(Copy, Clone, PartialEq, Eq)]
                pub enum Specifier {
                    $( $key, )*
                }

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
                    )*
                {
                    type Iter = ValueIter<$(
                        <super::$value as
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

                impl<T: 'static> $crate::params::HasParamStorage<T> for Specifier {
                    type Storage = ParamsWithExtra<T>;
                }

                impl<T: 'static> $crate::params::HasStorage<T> for Specifier {
                    type Storage = Storage<T>;
                }

                impl $crate::RuntimeSpecifier for Specifier {
                    const VALUES: &'static [Self] = &[ $( Specifier::$key ),* ];
                    // TODO: This should just be a stopgap until `const fn`s are more fleshed-out - at the
                    //       moment it's not possible to define this as `VALUES.map(Self::typeof)`.
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
                }

                $(
                    pub enum $key {}
                    impl $crate::params::Key for $key {
                        type Value = super::$value;
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
                pub struct ParamsWithExtra<T> {
                    params: Params,
                    extra: Extra<T>,
                }

                impl<T> Default for ParamsWithExtra<T> where T: Default, Params: Default, {
                    fn default() -> Self {
                        Self {
                            params: Default::default(),
                            extra: Default::default(),
                        }
                    }
                }

                #[allow(non_snake_case)]
                #[derive(Default)]
                struct Extra<T> {
                    $(
                        $key : T,
                    )*
                }

                #[allow(non_snake_case)]
                pub struct Params {
                    $(
                        pub $key : super::$value,
                    )*
                }

                const _: () = {
                    fn assert_params_implements_default() where Params: Default {}
                };

                $(
                    impl<T> $crate::params::ParamStorageGet<$key> for ParamsWithExtra<T> {
                        type Output = super::$value;
                        type Extra = T;

                        fn get(&self) -> (&Self::Output, &Self::Extra) {
                            (&self.params.$key, &self.extra.$key)
                        }

                        fn get_mut(&mut self) -> (&mut Self::Output, &mut Self::Extra) {
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

                #[derive(TryInto)]
                pub enum Ref<'a> {
                    $(
                        $key(&'a super::$value)
                    ),*
                }

                #[derive(TryInto)]
                pub enum RefMut<'a> {
                    $(
                        $key(&'a mut super::$value)
                    ),*
                }

                impl<'a, T: 'a> $crate::params::ParamStorage<'a> for ParamsWithExtra<T> {
                    type Ref = Ref<'a>;
                    type RefMut = RefMut<'a>;
                    type Extra = T;
                    type Specifier = Specifier;

                    fn get(&'a self, spec: Self::Specifier) -> (Self::Ref, &'a Self::Extra) {
                        match spec {
                            $(
                                Specifier::$key => {
                                    (Ref::$key(&self.params.$key), &self.extra.$key)
                                },
                            )*
                        }
                    }

                    fn get_mut(&'a mut self, spec: Self::Specifier) -> (Self::RefMut, &'a mut Self::Extra) {
                        match spec {
                            $(
                                Specifier::$key => {
                                    (RefMut::$key(&mut self.params.$key), &mut self.extra.$key)
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
    use super::*;
    use crate::*;

    specs! {
        mod foo {
            A: Value,
            B: MidiValue
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

    #[test]
    fn test_params() {
        type S = <foo::Specifier as HasParamStorage<bool>>::Storage;

        let mut storage = S::default();

        let (a, b) = <S as ParamStorageGet<foo::A>>::get_mut(&mut storage);
        *a = Value::saturating_from_num(-1);
        *b = true;
        let (a, b) = <S as ParamStorageGet<foo::B>>::get_mut(&mut storage);
        *a = MidiValue::ChannelPressure(1);
        *b = true;
        assert_eq!(
            <S as ParamStorageGet<foo::A>>::get(&storage),
            (&Value::saturating_from_num(-1), &true)
        );
        assert_eq!(
            <S as ParamStorageGet<foo::B>>::get(&storage),
            (&MidiValue::ChannelPressure(1), &true)
        );
    }
}
