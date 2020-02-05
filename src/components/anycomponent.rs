use crate::{
    components::PossiblyIter,
    context::ContextMeta,
    params::{ParamStorage, StorageMut},
    rack::{marker, InternalWire, Wire},
    RefRuntimeSpecifier, SpecId, Value, ValueType,
};
use nom_midi::MidiEventType;
use std::{any::Any, fmt};

#[macro_export]
macro_rules! component_set {
    ($v:vis mod $name:ident { $($t:ident),* }) => {
        #[allow(dead_code)]
        $v mod $name {
            use $crate::{Component as _, derive_more::TryInto};

            #[derive(Clone)]
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

                fn get(&self, spec: &Self::Specifier) -> &Self::Inner{
                    match self {
                        $(
                            Self::$t(inner) => {
                                inner.get(&$crate::RuntimeSpecifier::from_id(spec.0))
                            },
                        )*
                    }
                }
            }

            impl $crate::params::StorageMut for InputStorage {
                fn get_mut(&mut self, spec: &Self::Specifier) -> &mut Self::Inner{
                    match self {
                        $(
                            Self::$t(inner) => {
                                inner.get_mut(&$crate::RuntimeSpecifier::from_id(spec.0))
                            },
                        )*
                    }
                }
            }

            impl $crate::params::ParamStorage for ParamStorage {
                type Specifier = $crate::AnyParamSpec;

                fn get(&self, spec: &Self::Specifier) -> (&dyn std::any::Any, &dyn std::any::Any) {
                    match self {
                        $(
                            Self::$t(inner) => {
                                let (r, e) = inner.get(&$crate::RuntimeSpecifier::from_id(spec.0));

                                (r, e)
                            },
                        )*
                    }
                }

                fn get_mut(&mut self, spec: &Self::Specifier) -> (&mut dyn std::any::Any, &mut dyn std::any::Any) {
                    match self {
                        $(
                            Self::$t(inner) => {
                                let (r, e) = inner.get_mut(&$crate::RuntimeSpecifier::from_id(spec.0));

                                (r, e)
                            },
                        )*
                    }
                }
            }

            #[derive(Clone)]
            pub enum OneOf<$($t),*> {
                $( $t($t) ),*
            }

            impl<$($t),*> std::fmt::Display for OneOf<$($t),*>
            where $($t: std::fmt::Display),*
            {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    match self {
                        $( Self::$t(val) => val.fmt(f), )*
                    }
                }
            }

            impl<$($t),*, __V> Iterator for OneOf<$($t),*>
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

            impl<$($t),*, __V> std::iter::ExactSizeIterator for OneOf<$($t),*>
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

            #[derive(Clone)]
            pub enum ValueIter<$($t),*> {
                $( $t($t) ),*
            }

            impl<__Any, $($t),*> $crate::components::PossiblyIter<__Any> for ValueIter<$($t),*>
            where $($t: $crate::components::PossiblyIter<__Any>),*
            {
                type Iter = OneOf<$($t::Iter),*>;

                fn try_iter(self) -> Result<Self::Iter, Self> {
                    match self {
                        $(
                            Self::$t(inner) => inner.try_iter().map(OneOf::$t).map_err(ValueIter::$t),
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

            impl<'a> $crate::components::anycomponent::AnyUiElement<'a> for &'a Component
            where $( super::$t: $crate::UiElement),*
            {
                type InputNames = impl std::iter::ExactSizeIterator<Item = &'a dyn $crate::RefRuntimeSpecifier> + 'a;
                type OutputNames = impl std::iter::ExactSizeIterator<Item = &'a dyn $crate::RefRuntimeSpecifier> + 'a;
                type ParamNames = impl std::iter::ExactSizeIterator<Item = &'a dyn $crate::RefRuntimeSpecifier> + 'a;

                fn name(self) -> &'static str {
                    match self {
                        $(
                            Component::$t(_) => <super::$t as $crate::UiElement>::NAME,
                        )*
                    }
                }
                fn input_names(self) -> Self::InputNames {
                    match self {
                        $(
                            Component::$t(_) => {
                                <<super::$t as $crate::Component>::InputSpecifier as $crate::components::EnumerateValues>::values().map(|v| v as &dyn $crate::RefRuntimeSpecifier).collect::<Vec<_>>().into_iter()
                            },
                        )*
                    }
                }
                fn output_names(self) -> Self::OutputNames {
                    match self {
                        $(
                            Component::$t(_) => {
                                <<super::$t as $crate::Component>::OutputSpecifier as $crate::components::EnumerateValues>::values().map(|v| v as &dyn $crate::RefRuntimeSpecifier).collect::<Vec<_>>().into_iter()
                            },
                        )*
                    }
                }
                fn param_names(self) -> Self::ParamNames {
                    match self {
                        $(
                            Component::$t(_) => {
                                <<super::$t as $crate::Component>::ParamSpecifier as $crate::components::EnumerateValues>::values().map(|v| v as &dyn $crate::RefRuntimeSpecifier).collect::<Vec<_>>().into_iter()
                            },
                        )*
                    }
                }
            }

            impl<'a> $crate::components::anycomponent::AnyUiElementDisplayParamValue<'a> for &'a Component {
                type ParamStorage = ParamStorage;
                type Display = impl std::fmt::Display;

                fn display_param_value(
                    self,
                    spec: $crate::AnyParamSpec,
                    val: &dyn std::any::Any,
                ) -> Self::Display {
                    use $crate::{RuntimeSpecifier, params::DisplayParamValue};

                    match self {
                        $(
                            Component::$t(_) => OneOf::$t(
                                <
                                    super::$t as $crate::Component
                                >::ParamSpecifier::from_id(spec.0)
                                    .display(val),
                            ),
                        )*
                    }
                }
            }

            impl $crate::AnyComponent for Component
            where $( super::$t: $crate::Component ),*
            {
                type ParamStorage = ParamStorage;
                type InputStorage = InputStorage;
                type OutputIter = ValueIter<$($crate::params::OutputIterForComponent<super::$t>),*>;
                type Types = impl $crate::components::anycomponent::Types;

                fn types(&self) -> Self::Types {
                    struct QuickTypes<I, O, P> {
                        input: I,
                        output: O,
                        param: P,
                    }

                    impl<I, O, P> $crate::components::anycomponent::Types for QuickTypes<I, O, P>
                    where
                        I: ExactSizeIterator<Item = $crate::ValueType> + Clone,
                        O: ExactSizeIterator<Item = $crate::ValueType> + Clone,
                        P: ExactSizeIterator<Item = $crate::ValueType> + Clone,
                    {
                        type InputTypes = I;
                        type OutputTypes = O;
                        type ParamTypes = P;

                        fn input_types(&self) -> Self::InputTypes {
                            self.input.clone()
                        }

                        fn output_types(&self) -> Self::OutputTypes {
                            self.output.clone()
                        }

                        fn param_types(&self) -> Self::ParamTypes {
                            self.param.clone()
                        }
                    }

                    use $crate::RefRuntimeSpecifier;

                    match self {
                        $(
                            Self::$t(_) => {
                                QuickTypes {
                                    input: OneOf::$t(
                                        <
                                            <super::$t as $crate::Component>::InputSpecifier as
                                                $crate::components::EnumerateValues
                                        >::values().map(|v| v.value_type())
                                    ),
                                    output: OneOf::$t(
                                        <
                                            <super::$t as $crate::Component>::OutputSpecifier as
                                                $crate::components::EnumerateValues
                                        >::values().map(|v| v.value_type())
                                    ),
                                    param: OneOf::$t(
                                        <
                                            <super::$t as $crate::Component>::ParamSpecifier as
                                                $crate::components::EnumerateValues
                                        >::values().map(|v| v.value_type())
                                    ),
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

macro_rules! impl_spec_wrapper {
    ($t:ty, $name:expr) => {
        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}{}", $name, self.0)
            }
        }

        impl RefRuntimeSpecifier for $t {
            fn id(&self) -> SpecId {
                self.0
            }

            fn value_type(&self) -> ValueType {
                // TODO
                ValueType::continuous()
            }
        }

        impl crate::RuntimeSpecifier for $t {
            fn from_id(id: SpecId) -> Self {
                Self(id)
            }
        }
    };
}

impl_spec_wrapper!(AnyOutputSpec, "Output");
impl_spec_wrapper!(AnyInputSpec, "Input");
impl_spec_wrapper!(AnyParamSpec, "Param");

pub trait Types {
    type InputTypes: ExactSizeIterator<Item = ValueType>;
    type OutputTypes: ExactSizeIterator<Item = ValueType>;
    type ParamTypes: ExactSizeIterator<Item = ValueType>;

    fn input_types(&self) -> Self::InputTypes;
    fn output_types(&self) -> Self::OutputTypes;
    fn param_types(&self) -> Self::ParamTypes;
}

pub struct Names {
    pub input: &'static [ValueType],
    pub output: &'static [ValueType],
    pub parameters: &'static [ValueType],
}

pub trait AnyUiElement<'a> {
    type InputNames: ExactSizeIterator<Item = &'a dyn RefRuntimeSpecifier>;
    type OutputNames: ExactSizeIterator<Item = &'a dyn RefRuntimeSpecifier>;
    type ParamNames: ExactSizeIterator<Item = &'a dyn RefRuntimeSpecifier>;

    fn name(self) -> &'static str;
    fn input_names(self) -> Self::InputNames;
    fn output_names(self) -> Self::OutputNames;
    fn param_names(self) -> Self::ParamNames;
}

pub trait AnyUiElementDisplayParamValue<'a>: AnyUiElement<'a> {
    type ParamStorage;
    type Display: fmt::Display;

    fn display_param_value(self, spec: AnyParamSpec, value: &dyn Any) -> Self::Display;
}

pub trait AnyMeta {
    type ParamStorage;
    type InputStorage;

    fn params(&self) -> &Self::ParamStorage;
    fn inputs(&self) -> &Self::InputStorage;
}

pub trait AnyContext: AnyMeta {
    type Iter: PossiblyIter<Value> + PossiblyIter<MidiEventType>;

    fn read_wire(&self, wire: Wire<marker::Output>) -> Option<Self::Iter>;
}

pub trait AnyComponent: Sized {
    type ParamStorage: ParamStorage<Specifier = AnyParamSpec>;
    type InputStorage: StorageMut<Specifier = AnyInputSpec, Inner = InternalWire>;
    type Types: Types;

    type OutputIter: PossiblyIter<Value> + PossiblyIter<MidiEventType>;

    fn types(&self) -> Self::Types;

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
