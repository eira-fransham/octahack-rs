use crate::{GetInput, GetParam, SpecId, Value, ValueIter, ValueType};

#[macro_export]
macro_rules! component_set {
    ($v:vis mod $name:ident { $($t:ident),* }) => {
        #[allow(dead_code)]
        $v mod $name {
            use $crate::Component as _;

            pub enum Component {
                $($t(super::$t)),*
            }

            pub enum Input {
                $($t(<super::$t as $crate::Component>::InputSpecifier)),*
            }
            pub enum Output {
                $($t(<super::$t as $crate::Component>::OutputSpecifier)),*
            }
            pub enum Param {
                $($t(<super::$t as $crate::Component>::ParamSpecifier)),*
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

                #[allow(unreachable_code)]
                impl From<<super::$t as $crate::Component>::InputSpecifier> for Input {
                    fn from(other: <super::$t as $crate::Component>::InputSpecifier) -> Self {
                        Self::$t(other)
                    }
                }

                #[allow(unreachable_code)]
                impl From<<super::$t as $crate::Component>::OutputSpecifier> for Output {
                    fn from(other: <super::$t as $crate::Component>::OutputSpecifier) -> Self {
                        Self::$t(other)
                    }
                }

                #[allow(unreachable_code)]
                impl From<<super::$t as $crate::Component>::ParamSpecifier> for Param {
                    fn from(other: <super::$t as $crate::Component>::ParamSpecifier) -> Self {
                        Self::$t(other)
                    }
                }
            )*

            enum ParamDefaultsInner {
                $($t(&'static [<super::$t as $crate::Component>::ParamSpecifier])),*
            }

            pub struct ParamDefaults {
                inner: ParamDefaultsInner,
            }

            impl Iterator for ParamDefaults
            where
            $(
                <super::$t as $crate::Component>::ParamSpecifier: $crate::Param
            ),*
            {
                type Item = $crate::Value;

                fn next(&mut self) -> Option<Self::Item> {
                    match &mut self.inner {
                        $(
                            ParamDefaultsInner::$t(inner) => {
                                let (first, rest) = inner.split_first()?;

                                *inner = rest;

                                Some($crate::Param::default(first))
                            },
                        )*
                    }
                }
            }

            impl ExactSizeIterator for ParamDefaults {
                fn len(&self) -> usize {
                    match &self.inner {
                        $(
                            ParamDefaultsInner::$t(inner) => {
                                inner.len()
                            },
                        )*
                    }
                }
            }

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
                                        $crate::Specifier
                                >::VALUES.len()
                            };
                            // `0xFFFFFFFF` if count > out, 0 otherwise
                            let out_mask = (!(count > out) as usize).wrapping_sub(1);

                            out = (!out_mask & out) | (out_mask & count);
                        }
                    )*

                    out
                };

                type ParamDefaults = ParamDefaults;
                type OutputIter = ValueIter<$(<super::$t as $crate::Component>::OutputIter),*>;

                fn types(&self) -> $crate::Types {
                    match self {
                        $(
                            Self::$t(_) => {
                                $crate::Types {
                                input: <<super::$t as $crate::Component>::InputSpecifier as $crate::Specifier>::TYPES,
                                output: <<super::$t as $crate::Component>::OutputSpecifier as $crate::Specifier>::TYPES,
                                parameters:<<super::$t as $crate::Component>::ParamSpecifier as $crate::Specifier>::TYPES,
                                }
                            },
                        )*
                    }
                }

                fn param_defaults(&self) -> Self::ParamDefaults {
                    match self {
                        $(
                            Self::$t(_) => {
                                ParamDefaults {
                                    inner: ParamDefaultsInner::$t(
                                        <
                                            <super::$t as $crate::Component>::OutputSpecifier as
                                                $crate::Specifier
                                        >::VALUES
                                    )
                                }
                            },
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
                                use $crate::Specifier;

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
                                use $crate::Specifier;

                                ValueIter::$t(val.output(
                                    <<super::$t as $crate::Component>::OutputSpecifier as $crate::Specifier>::from_id(id.0),
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

    type ParamDefaults: ExactSizeIterator<Item = Value> + Send;
    type OutputIter: ValueIter + Send;

    fn types(&self) -> Types;

    fn param_defaults(&self) -> Self::ParamDefaults;

    // TODO: Support MIDI
    fn update<Ctx>(&self, ctx: Ctx) -> Self
    where
        Ctx: GetInput<AnyInputSpec> + GetParam<AnyParamSpec>;

    fn output<Ctx>(&self, id: AnyOutputSpec, ctx: Ctx) -> Self::OutputIter
    where
        Ctx: GetInput<AnyInputSpec> + GetParam<AnyParamSpec>;
}
