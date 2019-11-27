use crate::{
    params::{ParamStorage, Storage},
    rack::{marker, InternalParamWire, InternalWire, Wire},
    SpecId, ValueIter, ValueType,
};

#[macro_export]
macro_rules! component_set {
    ($v:vis mod $name:ident { $($t:ident),* }) => {
        #[allow(dead_code)]
        $v mod $name {
            use $crate::{Component as _, derive_more::TryInto};

            pub enum Component {
                $($t(super::$t)),*
            }

            #[derive(TryInto)]
            #[try_into(owned, ref, ref_mut)]
            pub enum ParamStorage {
                $($t(<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::HasParamStorage<$crate::rack::InternalParamWire>>::Storage)),*
            }

            #[derive(TryInto)]
            #[try_into(owned, ref, ref_mut)]
            pub enum InputStorage {
                $($t(<<super::$t as $crate::Component>::InputSpecifier as $crate::params::HasStorage<$crate::rack::InternalWire>>::Storage)),*
            }

            impl<'a> $crate::params::Storage<'a> for InputStorage {
                type Inner = $crate::rack::InternalWire;
                type Refs = InputRefs<'a>;
                type RefsMut = InputRefsMut<'a>;

                fn refs(&'a self) -> Self::Refs {
                    match self { $( Self::$t(val) => InputRefs::$t(val.refs()), )* }
                }
                fn refs_mut(&'a mut self) -> Self::RefsMut {
                    match self { $( Self::$t(val) => InputRefsMut::$t(val.refs_mut()), )* }
                }
            }

            impl<'a> $crate::params::ParamStorage<'a> for ParamStorage {
                type Extra = $crate::rack::InternalParamWire;
                type Ref = ParamRef<'a>;
                type RefMut = ParamRefMut<'a>;
                type Refs = ParamRefs<'a>;
                type RefsMut = ParamRefsMut<'a>;

                fn refs(&'a self) -> Self::Refs {
                    match self { $( Self::$t(val) => ParamRefs::$t(val.refs()), )* }
                }
                fn refs_mut(&'a mut self) -> Self::RefsMut {
                    match self { $( Self::$t(val) => ParamRefsMut::$t(val.refs_mut()), )* }
                }
            }

            pub enum InputRefs<'a> {
                $($t(<<<super::$t as $crate::Component>::InputSpecifier as $crate::params::HasStorage<$crate::rack::InternalWire>>::Storage as $crate::params::Storage<'a>>::Refs)),*
            }

            pub enum InputRefsMut<'a> {
                $($t(<<<super::$t as $crate::Component>::InputSpecifier as $crate::params::HasStorage<$crate::rack::InternalWire>>::Storage as $crate::params::Storage<'a>>::RefsMut)),*
            }

            impl<'a> Iterator for InputRefs<'a> {
                type Item = &'a $crate::rack::InternalWire;

                fn next(&mut self) -> Option<Self::Item> {
                    match self { $( Self::$t(val) => val.next(), )* }
                }
            }

            impl<'a> Iterator for InputRefsMut<'a> {
                type Item = &'a mut $crate::rack::InternalWire;

                fn next(&mut self) -> Option<Self::Item> {
                    match self { $( Self::$t(val) => val.next(), )* }
                }
            }

            pub enum ParamRefs<'a> {
                $($t(<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::HasParamStorage<$crate::rack::InternalParamWire>>::Storage as $crate::params::ParamStorage<'a>>::Refs)),*
            }

            pub enum ParamRefsMut<'a> {
                $($t(<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::HasParamStorage<$crate::rack::InternalParamWire>>::Storage as $crate::params::ParamStorage<'a>>::RefsMut)),*
            }

            impl<'a> Iterator for ParamRefs<'a> {
                type Item = (ParamRef<'a>, &'a $crate::rack::InternalParamWire);

                fn next(&mut self) -> Option<Self::Item> {
                    match self { $( Self::$t(val) => val.next().map(|(r, e)| (ParamRef::$t(r), e)), )* }
                }
            }

            impl<'a> Iterator for ParamRefsMut<'a> {
                type Item = (ParamRefMut<'a>, &'a mut $crate::rack::InternalParamWire);

                fn next(&mut self) -> Option<Self::Item> {
                    match self { $( Self::$t(val) => val.next().map(|(r, e)| (ParamRefMut::$t(r), e)), )* }
                }
            }

            pub enum ParamRef<'a> {
                $($t(<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::HasParamStorage<$crate::rack::InternalParamWire>>::Storage as $crate::params::ParamStorage<'a>>::Ref)),*
            }

            pub enum ParamRefMut<'a> {
                $($t(<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::HasParamStorage<$crate::rack::InternalParamWire>>::Storage as $crate::params::ParamStorage<'a>>::RefMut)),*
            }

            impl<'a> std::convert::TryFrom<ParamRef<'a>> for &'a $crate::Value {
                type Error = ((), ParamRef<'a>);

                fn try_from(other: ParamRef<'a>) -> Result<Self, Self::Error> {
                    match other { $( ParamRef::$t(val) => { <&'a $crate::Value>::try_from(val).map_err(|(_, v)| ((), ParamRef::$t(v))) }, )* }
                }
            }

            impl<'a> std::convert::TryFrom<ParamRefMut<'a>> for &'a mut $crate::Value {
                type Error = ((), ParamRefMut<'a>);

                fn try_from(other: ParamRefMut<'a>) -> Result<Self, Self::Error> {
                    match other { $( ParamRefMut::$t(val) => { <&'a mut $crate::Value>::try_from(val).map_err(|(_, v)| ((), ParamRefMut::$t(v))) }, )* }
                }
            }

            pub enum Iter<$($t),*> {
                $( $t($t) ),*
            }

            impl<$($t),*, __V> Iterator for Iter<$($t),*>
            where $($t: ExactSizeIterator<Item = __V>),*
            {
                type Item = __V;

                fn next(&mut self) -> Option<Self::Item> {
                    match self {
                        $(
                            Self::$t(inner) => Iterator::next(inner),
                        )*
                    }
                }

                fn size_hint(&self) -> (usize, Option<usize>) {
                    (self.len(), Some(self.len()))
                }
            }

            impl<$($t),*, __V> std::iter::ExactSizeIterator for Iter<$($t),*>
            where $($t: std::iter::ExactSizeIterator<Item = __V>),*
            {
                fn len(&self) -> usize {
                    match self {
                        $(
                            Self::$t(inner) => std::iter::ExactSizeIterator::len(inner),
                        )*
                    }
                }
            }

            pub enum ValueIter<$($t),*> {
                $( $t($t) ),*
            }

            impl<__Any, $($t),*> $crate::components::PossiblyIter<__Any> for ValueIter<$($t),*>
            where $($t: $crate::components::PossiblyIter<__Any>),*
            {
                type Iter = Iter<$($t::Iter),*>;

                fn try_iter(self) -> Result<Self::Iter, Self> {
                    match self {
                        $(
                            Self::$t(inner) => inner.try_iter().map(Iter::$t).map_err(ValueIter::$t),
                        )*
                    }
                }
            }

            $(
                impl From<super::$t> for Component {
                    fn from(other: super::$t) -> Self {
                        Component::$t(other)
                    }
                }
            )*

            impl $crate::AnyComponent for Component
            where $( super::$t: $crate::Component ),*
            {
                const MAX_OUTPUT_COUNT: usize = {
                    let mut out = 0;

                    $(
                        // This complex work is because we don't have access to most constructs in
                        // const contexts. FIXME when the Rust compiler implements what we need.
                        {
                            let count = {
                                <
                                    <super::$t as $crate::Component>::OutputSpecifier as
                                        $crate::RuntimeSpecifier
                                >::VALUES.len()
                            };
                            // `0xFFFFFFFF` if count > out, 0 otherwise
                            let out_mask = (!(count > out) as usize).wrapping_sub(1);

                            out = (!out_mask & out) | (out_mask & count);
                        }
                    )*

                    out
                };

                type ParamStorage = ParamStorage;
                type InputStorage = InputStorage;
                type OutputIter = ValueIter<$(<super::$t as $crate::Component>::OutputIter),*>;

                fn types(&self) -> $crate::Types {
                    match self {
                        $(
                            Self::$t(_) => {
                                $crate::Types {
                                input: <<super::$t as $crate::Component>::InputSpecifier as $crate::RuntimeSpecifier>::TYPES,
                                output: <<super::$t as $crate::Component>::OutputSpecifier as $crate::RuntimeSpecifier>::TYPES,
                                parameters:<<super::$t as $crate::Component>::ParamSpecifier as $crate::RuntimeSpecifier>::TYPES,
                                }
                            },
                        )*
                    }
                }

                fn param_default(&self) -> Self::ParamStorage {
                    match self {
                        $(
                            Self::$t(_) => ParamStorage::$t(Default::default()),
                        )*
                    }
                }

                fn input_default(&self) -> Self::InputStorage {
                    match self {
                        $(
                            Self::$t(_) => InputStorage::$t(Default::default()),
                        )*
                    }
                }

                #[allow(unreachable_code)]
                fn update<Ctx>(&self, ctx: &Ctx) -> Self
                where
                    Ctx: $crate::components::anycomponent::AnyContext<ParamStorage = Self::ParamStorage, InputStorage = Self::InputStorage>
                {
                    match self {
                        $(
                            Self::$t(val) => {
                                Self::$t(val.update(&$crate::context::Context::<_, super::$t>::new(ctx)))
                            },
                        )*
                    }
                }

                #[allow(unreachable_code)]
                fn output<Ctx>(&self, id: $crate::AnyOutputSpec, ctx: &Ctx) -> Self::OutputIter
                where
                    Ctx: $crate::components::anycomponent::AnyContext<ParamStorage = Self::ParamStorage, InputStorage = Self::InputStorage>
                {
                    match self {
                        $(
                            Self::$t(val) => {
                                use $crate::{RuntimeSpecifier, params::Output};

                                ValueIter::$t(
                                    <super::$t as $crate::Component>::OutputSpecifier::from_id(id.0).get_output(
                                        val,
                                        &crate::context::Context::<_, super::$t>::new(ctx),
                                    ),
                                )
                            },
                        )*
                    }
                }
            }
        }
    }
}

pub struct AnyOutputSpec(pub SpecId);
pub struct AnyInputSpec(pub SpecId);
pub struct AnyParamSpec(pub SpecId);

pub struct Types {
    pub input: &'static [ValueType],
    pub output: &'static [ValueType],
    pub parameters: &'static [ValueType],
}

pub trait AnyContext {
    type ParamStorage;
    type InputStorage;
    type Iter: ValueIter + Send;

    fn params(&self) -> &Self::ParamStorage;
    fn inputs(&self) -> &Self::InputStorage;
    fn read_wire(&self, wire: Wire<marker::Output>) -> Self::Iter;
}

pub trait AnyComponent: Sized {
    const MAX_OUTPUT_COUNT: usize;

    type ParamStorage: for<'a> ParamStorage<'a, Extra = InternalParamWire>;
    type InputStorage: for<'a> Storage<'a, Inner = InternalWire>;

    type OutputIter: ValueIter + Send;

    fn types(&self) -> Types;

    fn param_default(&self) -> Self::ParamStorage;
    fn input_default(&self) -> Self::InputStorage;

    fn update<Ctx>(&self, ctx: &Ctx) -> Self
    where
        Ctx: AnyContext<ParamStorage = Self::ParamStorage, InputStorage = Self::InputStorage>;

    fn output<Ctx>(&self, id: AnyOutputSpec, ctx: &Ctx) -> Self::OutputIter
    where
        Ctx: AnyContext<ParamStorage = Self::ParamStorage, InputStorage = Self::InputStorage>;
}
