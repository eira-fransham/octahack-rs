use crate::{
    components::PossiblyIter,
    context::ContextMeta,
    params::{ParamStorage, Storage},
    rack::{marker, InternalWire, Wire},
    SpecId, Value, ValueType,
};
use nom_midi::MidiEventType;

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
                $($t(<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::HasParamStorage>::Storage)),*
            }

            #[derive(TryInto)]
            #[try_into(owned, ref, ref_mut)]
            pub enum InputStorage {
                $($t(<<super::$t as $crate::Component>::InputSpecifier as $crate::params::HasStorage<$crate::rack::InternalWire>>::Storage)),*
            }

            impl $crate::params::Storage for InputStorage {
                type Inner = $crate::rack::InternalWire;
                type Specifier = $crate::AnyInputSpec;

                fn get(&self, spec: Self::Specifier) -> &Self::Inner{
                    match self {
                        $(
                            Self::$t(inner) => {
                                inner.get($crate::RuntimeSpecifier::from_id(spec.0))
                            },
                        )*
                    }
                }

                fn get_mut(&mut self, spec: Self::Specifier) -> &mut Self::Inner{
                    match self {
                        $(
                            Self::$t(inner) => {
                                inner.get_mut($crate::RuntimeSpecifier::from_id(spec.0))
                            },
                        )*
                    }
                }
            }

            impl $crate::params::ParamStorage for ParamStorage {
                type Specifier = $crate::AnyParamSpec;

                fn get(&self, spec: Self::Specifier) -> (&dyn std::any::Any, &dyn std::any::Any) {
                    match self {
                        $(
                            Self::$t(inner) => {
                                let (r, e) = inner.get($crate::RuntimeSpecifier::from_id(spec.0));

                                (r, e)
                            },
                        )*
                    }
                }

                fn get_mut(&mut self, spec: Self::Specifier) -> (&mut dyn std::any::Any, &mut dyn std::any::Any) {
                    match self {
                        $(
                            Self::$t(inner) => {
                                let (r, e) = inner.get_mut($crate::RuntimeSpecifier::from_id(spec.0));

                                (r, e)
                            },
                        )*
                    }
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
                type OutputIter = ValueIter<$($crate::params::OutputIterForComponent<super::$t>),*>;

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
                    Ctx: $crate::components::anycomponent::AnyContext<ParamStorage = Self::ParamStorage, InputStorage = Self::InputStorage> + $crate::context::ContextMeta
                {
                    match self {
                        $(
                            Self::$t(val) => {
                                Self::$t(val.update(&$crate::context::ContextForComponent::<_, super::$t>::new(ctx)))
                            },
                        )*
                    }
                }

                #[allow(unreachable_code)]
                fn output<Ctx>(&self, id: $crate::AnyOutputSpec, ctx: &Ctx) -> Self::OutputIter
                where
                    Ctx: $crate::components::anycomponent::AnyContext<ParamStorage = Self::ParamStorage, InputStorage = Self::InputStorage> + $crate::context::ContextMeta
                {
                    match self {
                        $(
                            Self::$t(val) => {
                                use $crate::{RuntimeSpecifier, params::Output};

                                ValueIter::$t(
                                    <super::$t as $crate::Component>::OutputSpecifier::from_id(id.0).get_output(
                                        val,
                                        &$crate::context::ContextForComponent::<_, super::$t>::new(ctx),
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
    type Iter: PossiblyIter<Value> + PossiblyIter<MidiEventType>;

    fn params(&self) -> &Self::ParamStorage;
    fn inputs(&self) -> &Self::InputStorage;
    fn read_wire(&self, wire: Wire<marker::Output>) -> Self::Iter;
}

pub trait AnyComponent: Sized {
    const MAX_OUTPUT_COUNT: usize;

    type ParamStorage: ParamStorage<Specifier = AnyParamSpec>;
    type InputStorage: Storage<Specifier = AnyInputSpec, Inner = InternalWire>;

    type OutputIter: PossiblyIter<Value> + PossiblyIter<MidiEventType>;

    fn types(&self) -> Types;

    fn param_default(&self) -> Self::ParamStorage;
    fn input_default(&self) -> Self::InputStorage;

    fn update<Ctx>(&self, ctx: &Ctx) -> Self
    where
        Ctx: AnyContext<ParamStorage = Self::ParamStorage, InputStorage = Self::InputStorage>
            + ContextMeta;

    fn output<Ctx>(&self, id: AnyOutputSpec, ctx: &Ctx) -> Self::OutputIter
    where
        Ctx: AnyContext<ParamStorage = Self::ParamStorage, InputStorage = Self::InputStorage>
            + ContextMeta;
}
