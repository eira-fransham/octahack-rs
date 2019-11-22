use crate::{
    components::Update, context::GetRuntimeParam, params::ParamStorage, Extra, GetInput, GetParam,
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
                $($t(<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::HasParamStorage<$crate::Extra>>::Storage)),*
            }

            impl<'a> $crate::params::ParamStorage<'a> for ParamStorage {
                type Extra = $crate::Extra;
                type Ref = Ref<'a>;
                type RefMut = RefMut<'a>;
                type Refs = Refs<'a>;
                type RefsMut = RefsMut<'a>;

                fn refs(&'a self) -> Self::Refs {
                    match self { $( Self::$t(val) => Refs::$t(val.refs()), )* }
                }
                fn refs_mut(&'a mut self) -> Self::RefsMut {
                    match self { $( Self::$t(val) => RefsMut::$t(val.refs_mut()), )* }
                }
            }

            pub enum Refs<'a> {
                $($t(<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::HasParamStorage<$crate::Extra>>::Storage as $crate::params::ParamStorage<'a>>::Refs)),*
            }

            pub enum RefsMut<'a> {
                $($t(<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::HasParamStorage<$crate::Extra>>::Storage as $crate::params::ParamStorage<'a>>::RefsMut)),*
            }

            impl<'a> Iterator for Refs<'a> {
                type Item = (Ref<'a>, &'a $crate::Extra);

                fn next(&mut self) -> Option<Self::Item> {
                    match self { $( Self::$t(val) => val.next().map(|(r, e)| (Ref::$t(r), e)), )* }
                }
            }

            impl<'a> Iterator for RefsMut<'a> {
                type Item = (RefMut<'a>, &'a mut $crate::Extra);

                fn next(&mut self) -> Option<Self::Item> {
                    match self { $( Self::$t(val) => val.next().map(|(r, e)| (RefMut::$t(r), e)), )* }
                }
            }

            pub enum Ref<'a> {
                $($t(<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::HasParamStorage<$crate::Extra>>::Storage as $crate::params::ParamStorage<'a>>::Ref)),*
            }

            pub enum RefMut<'a> {
                $($t(<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::HasParamStorage<$crate::Extra>>::Storage as $crate::params::ParamStorage<'a>>::RefMut)),*
            }

            impl<'a> std::convert::TryFrom<Ref<'a>> for &'a $crate::Value {
                type Error = ((), Ref<'a>);

                fn try_from(other: Ref<'a>) -> Result<Self, Self::Error> {
                    match other { $( Ref::$t(val) => { <&'a $crate::Value>::try_from(val).map_err(|(_, v)| ((), Ref::$t(v))) }, )* }
                }
            }

            impl<'a> std::convert::TryFrom<RefMut<'a>> for &'a mut $crate::Value {
                type Error = ((), RefMut<'a>);

                fn try_from(other: RefMut<'a>) -> Result<Self, Self::Error> {
                    match other { $( RefMut::$t(val) => { <&'a mut $crate::Value>::try_from(val).map_err(|(_, v)| ((), RefMut::$t(v))) }, )* }
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
            }

            impl<Ctx> $crate::components::Update<Ctx> for Component
            where
                $(
                    Ctx: $crate::components::anycomponent::FlippedUpdate<super::$t>,
                )*
            {
                #[allow(unreachable_code)]
                fn update(&self, ctx: Ctx) -> Self {
                    match self {
                        $(
                            Self::$t(val) => {
                                Self::$t(ctx.update(val))
                            },
                        )*
                    }
                }
            }

            impl<Ctx> $crate::components::anycomponent::AnyComponentOutput<Ctx> for Component
            where
                $(
                    Ctx: $crate::params::Output<super::$t, <super::$t as $crate::components::Component>::OutputSpecifier>,
                )*
            {
                #[allow(unreachable_code)]
                fn output(&self, id: $crate::AnyOutputSpec, ctx: Ctx) -> Self::OutputIter {
                    match self {
                        $(
                            Self::$t(val) => {
                                use $crate::RuntimeSpecifier;

                                ValueIter::$t(Ctx::get_output(ctx, val, <super::$t as $crate::Component>::OutputSpecifier::from_id(id.0)))
                            },
                        )*
                    }
                }
            }
        }
    }
}

pub trait FlippedUpdate<C> {
    fn update(self, component: &C) -> C;
}

impl<Ctx, C> FlippedUpdate<C> for Ctx
where
    C: Update<Ctx>,
{
    fn update(self, component: &C) -> C {
        component.update(self)
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

pub trait AnyComponent {
    const MAX_OUTPUT_COUNT: usize;

    type ParamStorage: for<'a> ParamStorage<'a, Extra = Extra>;

    type OutputIter: ValueIter + Send;

    fn types(&self) -> Types;

    fn param_default(&self) -> Self::ParamStorage;
}

pub trait AnyComponentOutput<Ctx>: AnyComponent {
    fn output(&self, id: AnyOutputSpec, ctx: Ctx) -> Self::OutputIter;
}
