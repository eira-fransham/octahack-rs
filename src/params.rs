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

impl<'a, A, B, E> Possibly<A> for B
where
    A: TryFrom<B, Error = (E, B)>,
{
    fn when_matches<F, O>(self, f: F) -> Result<O, Self>
    where
        F: FnOnce(A) -> O,
    {
        <A>::try_from(self).map(f).map_err(|(_, this)| this)
    }
}

pub trait ParamStorage<'a> {
    type Refs: Iterator + 'a;
    type RefsMut: Iterator + 'a;

    fn refs(&'a self) -> Self::Refs;
    fn refs_mut(&'a mut self) -> Self::RefsMut;
}

pub trait GenericStorage<'a, T: 'a> {
    type Refs: Iterator<Item = &'a T> + 'a;
    type RefsMut: Iterator<Item = &'a mut T> + 'a;

    fn refs(&'a self) -> Self::Refs;
    fn refs_mut(&'a mut self) -> Self::RefsMut;
}

pub trait WireStorage<'a>: Default {
    type Refs: Iterator<Item = &'a Option<crate::rack::WireSrc>> + 'a;
    type RefsMut: Iterator<Item = &'a Option<crate::rack::WireSrc>> + 'a;

    fn refs(&'a self) -> Self::Refs;
    fn refs_mut(&'a mut self) -> Self::RefsMut;
}

pub trait Output<C>
where
    C: crate::Component,
{
    fn get_output<Ctx>(self, comp: &C, ctx: Ctx) -> C::OutputIter
    where
        Ctx: crate::GetInput<C::InputSpecifier> + crate::GetParam<C::ParamSpecifier>;
}

pub trait Specifier {
    type ParamStorage;
}

pub trait GenericSpecifier<T>: Specifier {
    type Storage: for<'a> GenericStorage<'a, T>;
}

impl Specifier for ! {
    type ParamStorage = ();
}

pub trait HasParam<V> {
    type Output;

    fn get(&self) -> &Self::Output;
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

            impl $crate::params::Specifier for Specifier {
                type ParamStorage = Params;
            }

            impl<T: Default + 'static> $crate::params::GenericSpecifier<T> for Specifier {
                type Storage = GenericStorage<T>;
            }

            impl<C> $crate::params::Output<C> for Specifier
            where $(C: $crate::components::GetOutput<$key>),*
            {
                fn get_output<Ctx>(self, comp: &C, ctx: Ctx) -> C::OutputIter
                where
                    Ctx: $crate::GetInput<C::InputSpecifier> + $crate::GetParam<C::ParamSpecifier>
                {
                    match self {
                        $(
                            Self::$key => <C as $crate::components::GetOutput<$key>>::output(comp, ctx),
                        )*
                    }
                }
            }

            impl $crate::RuntimeSpecifier for Specifier {
                const VALUES: &'static [Self] = &[ $( Specifier::$key ),* ];
                // TODO: This should just be a stopgap until `const fn`s are more fleshed-out - at the
                //       moment it's not possible to define this as `VALUES.map(Self::typeof)`.
                const TYPES: &'static [$crate::ValueType] = &[
                    $( (stringify!($key), $crate::ValueType::mono()).1 ),*
                ];

                fn id(&self) -> $crate::SpecId {
                    for (i, val) in Self::VALUES.iter().enumerate() {
                        if self == val { return i; }
                    }

                    unreachable!()
                }
            }

            $( pub enum $key {} )*

            #[allow(non_snake_case)]
            #[derive(Default)]
            pub struct GenericStorage<T> {
                $(
                    pub $key : T,
                )*
            }

            impl<'a, T: Default + 'a> $crate::params::GenericStorage<'a, T> for GenericStorage<T> {
                type Refs = $crate::params::FixedSizeArrayIter<[&'a T; $( (stringify!($key), 1).1 + )*0]>;
                type RefsMut = $crate::params::FixedSizeArrayIter<[&'a mut T; $( (stringify!($key), 1).1 + )*0]>;

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
            pub struct Params {
                $(
                    pub $key : super::$value,
                )*
            }

            $(
                impl $crate::params::HasParam<$key> for Params {
                    type Output = super::$value;

                    fn get(&self) -> &Self::Output {
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

            impl<'a> $crate::params::ParamStorage<'a> for Params {
                type Refs = $crate::params::FixedSizeArrayIter<[Ref<'a>; $( (stringify!($key), 1).1 + )*0]>;
                type RefsMut = $crate::params::FixedSizeArrayIter<[RefMut<'a>; $( (stringify!($key), 1).1 + )*0]>;

                fn refs(&'a self) -> Self::Refs {
                    $crate::params::FixedSizeArrayIter::new([
                        $(
                            Ref::$key(&self.$key)
                        ),*
                    ])
                }

                fn refs_mut(&'a mut self) -> Self::RefsMut {
                    $crate::params::FixedSizeArrayIter::new([
                        $(
                            RefMut::$key(&mut self.$key)
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
        let mut storage = <foo::Specifier as Specifier>::ParamStorage::default();

        for (i, val) in storage.refs_mut().enumerate() {
            val.when_matches(|val: &mut u32| *val = i as _)
                .or_else(|val| val.when_matches(|val: &mut u64| *val = i as _))
                .unwrap_or_else(|_| unreachable!())
        }

        for (i, val) in storage.refs().enumerate() {
            val.when_matches(|val: &u32| assert_eq!(*val, i as u32))
                .or_else(|val| val.when_matches(|val: &u64| assert_eq!(*val, i as u64)))
                .unwrap_or_else(|_| unreachable!())
        }
    }
}
