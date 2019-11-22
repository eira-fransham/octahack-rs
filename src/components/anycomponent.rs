use crate::{
    params::{GenericStorage, ParamStorage},
    rack::{InternalParamWire, InternalWire},
    GetInput, GetParam, SpecId, ValueIter, ValueType,
};

#[macro_export]
macro_rules! component_set {
    ($v:vis mod $name:ident { $($t:ident),* }) => {
        #[allow(dead_code)]
        $v mod $name {
            use $crate::Component as _;

            pub enum Component {
                $($t(super::$t)),*
            }

            pub enum GenericStorage<T>
            where
            $(
                <super::$t as $crate::Component>::ParamSpecifier: $crate::params::GenericSpecifier<T>
            ),*
            {
                $($t(<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::GenericSpecifier<T>>::Storage)),*
            }

            impl<'a, T: Default + 'a> $crate::params::GenericStorage<'a, T> for GenericStorage<T>
            where
            $(
                <super::$t as $crate::Component>::ParamSpecifier: $crate::params::GenericSpecifier<T>
            ),*
            {
                type Refs = RefsGeneric<'a, T>;
                type RefsMut = RefsMutGeneric<'a, T>;

                fn refs(&'a self) -> Self::Refs {
                    match self { $( Self::$t(val) => RefsGeneric::$t(val.refs()), )* }
                }

                fn refs_mut(&'a mut self) -> Self::RefsMut {
                    match self { $( Self::$t(val) => RefsMutGeneric::$t(val.refs_mut()), )* }
                }
            }

            pub enum RefsGeneric<'a, T>
            where
            $(
                <super::$t as $crate::Component>::ParamSpecifier: $crate::params::GenericSpecifier<T>
            ),*
            {
                $($t(<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::GenericSpecifier<T>>::Storage as $crate::params::GenericStorage<'a, T>>::Refs)),*
            }

            pub enum RefsMutGeneric<'a, T>
            where
            $(
                <super::$t as $crate::Component>::ParamSpecifier: $crate::params::GenericSpecifier<T>
            ),*
            {
                $($t(<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::GenericSpecifier<T>>::Storage as $crate::params::GenericStorage<'a, T>>::RefsMut)),*
            }

            impl<'a, T: Default + 'a> Iterator for RefsGeneric<'a, T> {
                type Item = &'a T;

                fn next(&mut self) -> Option<Self::Item> {
                    match self { $( Self::$t(val) => val.next(), )* }
                }
            }

            impl<'a, T: Default + 'a> Iterator for RefsMutGeneric<'a, T> {
                type Item = &'a mut T;

                fn next(&mut self) -> Option<Self::Item> {
                    match self { $( Self::$t(val) => val.next(), )* }
                }
            }

            pub enum ParamStorage {
                $($t(<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::Specifier>::ParamStorage)),*
            }

            impl<'a> $crate::params::ParamStorage<'a> for ParamStorage {
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
                $($t(<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::Specifier>::ParamStorage as $crate::params::ParamStorage<'a>>::Refs)),*
            }

            pub enum RefsMut<'a> {
                $($t(<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::Specifier>::ParamStorage as $crate::params::ParamStorage<'a>>::RefsMut)),*
            }

            impl<'a> Iterator for Refs<'a> {
                type Item = Ref<'a>;

                fn next(&mut self) -> Option<Ref<'a>> {
                    match self { $( Self::$t(val) => val.next().map(Ref::$t), )* }
                }
            }

            impl<'a> Iterator for RefsMut<'a> {
                type Item = RefMut<'a>;

                fn next(&mut self) -> Option<RefMut<'a>> {
                    match self { $( Self::$t(val) => val.next().map(RefMut::$t), )* }
                }
            }

            pub enum Ref<'a> {
                $($t(<<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::Specifier>::ParamStorage as $crate::params::ParamStorage<'a>>::Refs as Iterator>::Item)),*
            }

            pub enum RefMut<'a> {
                $($t(<<<<super::$t as $crate::Component>::ParamSpecifier as $crate::params::Specifier>::ParamStorage as $crate::params::ParamStorage<'a>>::RefsMut as Iterator>::Item)),*
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

            impl<$($t),*> $crate::ValueIter for ValueIter<$($t),*>
            where $($t: $crate::ValueIter),*
            {
                type Midi = Iter<$($t::Midi),*>;
                type Analog = Iter<$($t::Analog),*>;

                fn midi(self) -> Option<Self::Midi> {
                    match self {
                        $(
                            Self::$t(inner) => inner.midi().map(Iter::$t),
                        )*
                    }
                }

                fn analog(self) -> Option<Self::Analog> {
                    match self {
                        $(
                            Self::$t(inner) => inner.analog().map(Iter::$t),
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
                type InputWireStorage = GenericStorage<$crate::rack::InternalWire>;
                type ParamWireStorage = GenericStorage<$crate::rack::InternalParamWire>;
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

                fn input_wires_default(&self) -> Self::InputWireStorage {
                    match self {
                        $(
                            Self::$t(_) => Self::InputWireStorage::$t(Default::default()),
                        )*
                    }
                }

                fn param_wires_default(&self) -> Self::ParamWireStorage {
                    match self {
                        $(
                            Self::$t(_) => Self::ParamWireStorage::$t(Default::default()),
                        )*
                    }
                }

                #[allow(unreachable_code)]
                fn update<Ctx>(&self, ctx: Ctx) -> Self
                where
                    Ctx: $crate::GetInput<$crate::AnyInputSpec> + $crate::GetParam<$crate::AnyParamSpec>
                {
                    match self {
                        $(
                            Self::$t(val) => {
                                use $crate::RuntimeSpecifier;

                                Self::$t(val.update(
                                    $crate::QuickContext::new(
                                        ctx,
                                        |ctx: &Ctx, spec: <super::$t as $crate::Component>::InputSpecifier| {
                                            ctx.input($crate::AnyInputSpec(spec.id()))
                                        },
                                        |ctx: &Ctx, spec: <super::$t as $crate::Component>::ParamSpecifier| ctx.param($crate::AnyParamSpec(spec.id())),
                                    )
                                ))
                            },
                        )*
                    }
                }

                #[allow(unreachable_code)]
                fn output<Ctx>(&self, id: $crate::AnyOutputSpec, ctx: Ctx) -> Self::OutputIter
                where
                    Ctx: $crate::GetInput<$crate::AnyInputSpec> + $crate::GetParam<$crate::AnyParamSpec>
                {
                    match self {
                        $(
                            Self::$t(val) => {
                                use $crate::{RuntimeSpecifier, params::Output};

                                ValueIter::$t(<<super::$t as $crate::Component>::OutputSpecifier as $crate::RuntimeSpecifier>::from_id(id.0).get_output(
                                    val,
                                    $crate::QuickContext::new(
                                        ctx,
                                        |ctx: &Ctx, spec: <super::$t as $crate::Component>::InputSpecifier| {
                                            ctx.input($crate::AnyInputSpec(spec.id()))
                                        },
                                        |ctx: &Ctx, spec: <super::$t as $crate::Component>::ParamSpecifier| ctx.param($crate::AnyParamSpec(spec.id())),
                                    )
                                ))
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

pub trait AnyComponent {
    const MAX_OUTPUT_COUNT: usize;

    type ParamStorage: for<'a> ParamStorage<'a>;
    type InputWireStorage: for<'a> GenericStorage<'a, InternalWire>;
    type ParamWireStorage: for<'a> GenericStorage<'a, InternalParamWire>;
    type OutputIter: ValueIter + Send;

    fn types(&self) -> Types;

    fn param_default(&self) -> Self::ParamStorage;
    fn input_wires_default(&self) -> Self::InputWireStorage;
    fn param_wires_default(&self) -> Self::ParamWireStorage;

    // TODO: Support MIDI
    fn update<Ctx>(&self, ctx: Ctx) -> Self
    where
        Ctx: GetInput<AnyInputSpec> + GetParam<AnyParamSpec>;

    fn output<Ctx>(&self, id: AnyOutputSpec, ctx: Ctx) -> Self::OutputIter
    where
        Ctx: GetInput<AnyInputSpec> + GetParam<AnyParamSpec>;
}
