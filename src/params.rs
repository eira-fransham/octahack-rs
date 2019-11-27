use crate::{GetInput, GetParam};
use arrayvec::Array;
use std::convert::TryFrom;

pub struct FixedSizeArrayIter<A>
where
    A: Array,
{
    inner: std::mem::ManuallyDrop<A>,
    index: usize,
}

impl<A> FixedSizeArrayIter<A>
where
    A: Array,
{
    pub fn new(arr: A) -> Self {
        Self {
            inner: std::mem::ManuallyDrop::new(arr),
            index: 0,
        }
    }
}

impl<A> Iterator for FixedSizeArrayIter<A>
where
    A: Array,
{
    type Item = A::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        if index < A::CAPACITY {
            unsafe {
                Some(std::ptr::read(
                    self.inner
                        .as_mut_slice()
                        .as_mut_ptr()
                        .offset(index as isize),
                ))
            }
        } else {
            None
        }
    }

    fn nth(&mut self, i: usize) -> Option<Self::Item> {
        if i >= self.index {
            self.index = i;
            self.next()
        } else {
            None
        }
    }
}

impl<A> Drop for FixedSizeArrayIter<A>
where
    A: Array,
{
    fn drop(&mut self) {
        // Drops all remaining elements
        for _ in self {}
    }
}

pub trait Possibly<T>: Sized {
    fn when_matches<F, O>(self, f: F) -> Result<O, Self>
    where
        F: FnOnce(T) -> O;
}

pub trait PossiblyHelper<B> {
    type Error;

    fn to_tuple(self) -> (Self::Error, B);
}

impl<'a, A, B> Possibly<A> for B
where
    A: TryFrom<B>,
    A::Error: PossiblyHelper<B>,
{
    fn when_matches<F, O>(self, f: F) -> Result<O, Self>
    where
        F: FnOnce(A) -> O,
    {
        <A>::try_from(self)
            .map(f)
            .map_err(PossiblyHelper::to_tuple)
            .map_err(|(_, this)| this)
    }
}

impl<B, E> PossiblyHelper<B> for (E, B) {
    type Error = E;

    fn to_tuple(self) -> (Self::Error, B) {
        self
    }
}

impl<B> PossiblyHelper<B> for std::convert::Infallible {
    type Error = !;

    fn to_tuple(self) -> (Self::Error, B) {
        unreachable!()
    }
}

pub trait ParamStorage<'a> {
    type Ref;
    type RefMut;
    type Extra: 'a;
    type Refs: Iterator<Item = (Self::Ref, &'a Self::Extra)> + 'a;
    type RefsMut: Iterator<Item = (Self::RefMut, &'a mut Self::Extra)> + 'a;

    fn refs(&'a self) -> Self::Refs;
    fn refs_mut(&'a mut self) -> Self::RefsMut;
}

pub trait Storage<'a> {
    type Inner: 'a;

    type Refs: Iterator<Item = &'a Self::Inner> + 'a;
    type RefsMut: Iterator<Item = &'a mut Self::Inner> + 'a;

    fn refs(&'a self) -> Self::Refs;
    fn refs_mut(&'a mut self) -> Self::RefsMut;
}

pub trait Output<C>
where
    C: crate::Component,
{
    fn get_output<Ctx>(self, comp: &C, ctx: &Ctx) -> C::OutputIter
    where
        Ctx: GetInput<C::InputSpecifier> + GetParam<C::ParamSpecifier>;
}

pub trait Specifier {}

pub trait HasStorage<T> {
    type Storage: for<'a> Storage<'a, Inner = T>;
}

pub trait HasParamStorage<T>: Specifier {
    type Storage: for<'a> ParamStorage<'a, Extra = T>;
}

impl Specifier for ! {}
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
    type Extra = T;
    type Refs = std::iter::Empty<(Self::Ref, &'a Self::Extra)>;
    type RefsMut = std::iter::Empty<(Self::RefMut, &'a mut Self::Extra)>;

    fn refs(&'a self) -> Self::Refs {
        std::iter::empty()
    }

    fn refs_mut(&'a mut self) -> Self::RefsMut {
        std::iter::empty()
    }
}

impl<'a, V: 'a> Storage<'a> for EmptyStorage<V> {
    type Inner = V;
    type Refs = std::iter::Empty<&'a Self::Inner>;
    type RefsMut = std::iter::Empty<&'a mut Self::Inner>;

    fn refs(&'a self) -> Self::Refs {
        std::iter::empty()
    }

    fn refs_mut(&'a mut self) -> Self::RefsMut {
        std::iter::empty()
    }
}

pub trait ParamStorageGet<V> {
    type Output;
    type Extra;

    fn get(&self) -> &(Self::Output, Self::Extra);
}

pub trait StorageGet<V>: for<'a> Storage<'a> {
    fn get<'a>(&'a self) -> &'a <Self as Storage<'a>>::Inner;
}

pub trait Key {
    type Value;
}

#[macro_export]
macro_rules! specs {
    ($v:vis mod $modname:ident { $($key:ident : $value:ident),* }) => {
        $v mod $modname {
            use $crate::derive_more::TryInto;

            #[derive(Copy, Clone, PartialEq, Eq)]
            pub enum Specifier {
                $( $key, )*
            }

            impl<C> $crate::params::Output<C> for Specifier
            where
                C: $crate::Component,
                $(
                    C: $crate::GetOutput<$key>,
                )*
            {
                fn get_output<Ctx>(self, comp: &C, ctx: &Ctx) -> C::OutputIter
                where
                    Ctx: $crate::GetInput<C::InputSpecifier> + $crate::GetParam<C::ParamSpecifier>
                    {
                        match self {
                            $(
                                Self::$key => <C as $crate::GetOutput<$key>>::output(comp, ctx),
                            )*
                        }
                    }
            }

            impl $crate::params::Specifier for Specifier {}

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
                    pub $key : V,
                )*
            }

            impl<'a, V: 'a> $crate::params::Storage<'a> for Storage<V> {
                type Inner = V;
                type Refs = $crate::params::FixedSizeArrayIter<[&'a V; $( (stringify!($key), 1).1 + )*0]>;
                type RefsMut = $crate::params::FixedSizeArrayIter<[&'a mut V; $( (stringify!($key), 1).1 + )*0]>;

                fn refs(&'a self) -> Self::Refs {
                    $crate::params::FixedSizeArrayIter::new([
                        $(
                            &self.$key
                        ),*
                    ])
                }

                fn refs_mut(&'a mut self) -> Self::RefsMut {
                    $crate::params::FixedSizeArrayIter::new([
                        $(
                            &mut self.$key
                        ),*
                    ])
                }
            }

            #[allow(non_snake_case)]
            pub struct ParamsWithExtra<T> {
                $(
                    pub $key : (super::$value, T),
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

            impl<T: Default> Default for ParamsWithExtra<T> {
                #[allow(non_snake_case)]
                fn default() -> Self {
                    let Params { $( $key ),* } = Params::default();
                    ParamsWithExtra { $( $key: ($key, Default::default()) ),* }
                }
            }

            $(
                impl<T> $crate::params::ParamStorageGet<$key> for ParamsWithExtra<T> {
                    type Output = super::$value;
                    type Extra = T;

                    fn get(&self) -> &(Self::Output, Self::Extra) {
                        &self.$key
                    }
                }

                impl<T: 'static> $crate::params::StorageGet<$key> for Storage<T> {
                    fn get<'a>(&'a self) -> &'a T {
                        &self.$key
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
                type Refs = $crate::params::FixedSizeArrayIter<[(Ref<'a>, &'a T); $( (stringify!($key), 1).1 + )*0]>;
                type RefsMut = $crate::params::FixedSizeArrayIter<[(RefMut<'a>, &'a mut T); $( (stringify!($key), 1).1 + )*0]>;

                fn refs(&'a self) -> Self::Refs {
                    $crate::params::FixedSizeArrayIter::new([
                        $(
                            { let (val, extra) = &self.$key; (Ref::$key(val), extra) }
                        ),*
                    ])
                }

                fn refs_mut(&'a mut self) -> Self::RefsMut {
                    $crate::params::FixedSizeArrayIter::new([
                        $(
                            { let (val, extra) = &mut self.$key; (RefMut::$key(val), extra) }
                        ),*
                    ])
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use {u32, u64};

    specs! {
        mod foo {
            A: u32,
            B: u64
        }
    }

    impl Default for foo::Params {
        fn default() -> Self {
            Self { A: 0, B: 0 }
        }
    }

    #[test]
    fn test_params() {
        let mut storage = <foo::Specifier as HasParamStorage<()>>::Storage::default();

        for (i, (val, ())) in storage.refs_mut().enumerate() {
            val.when_matches(|val: &mut u32| *val = i as _)
                .or_else(|val| val.when_matches(|val: &mut u64| *val = i as _))
                .unwrap_or_else(|_| unreachable!())
        }

        for (i, (val, ())) in storage.refs().enumerate() {
            val.when_matches(|val: &u32| assert_eq!(*val, i as u32))
                .or_else(|val| val.when_matches(|val: &u64| assert_eq!(*val, i as u64)))
                .unwrap_or_else(|_| unreachable!())
        }
    }
}
