use crate::{AnyIter, Component, GetInput, GetOutput, GetParam, Value, ValueExt, ValueIter};
use az::Az;

crate::specs! {
    mod amplifier {
        Only: Value
    }
}

use self::amplifier::Only;
pub use self::amplifier::Specifier;

impl Default for self::amplifier::Params {
    fn default() -> Self {
        Self {
            Only: Default::default(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct Amplifier;

type OutputIter = impl ValueIter + Send;

impl Component for Amplifier {
    type InputSpecifier = Specifier;
    type OutputSpecifier = Specifier;
    type ParamSpecifier = Specifier;
    type OutputIter = OutputIter;

    fn update<Ctx>(&self, _: &Ctx) -> Self
    where
        Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::OutputSpecifier>,
    {
        *self
    }
}

impl GetOutput<self::amplifier::Only> for Amplifier {
    fn output<Ctx>(&self, ctx: &Ctx) -> OutputIter
    where
        Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::OutputSpecifier>,
    {
        AnyIter::from(
            ctx.input::<Only>()
                .map(|inputs| {
                    {
                        inputs
                            .analog()
                            .unwrap()
                            .map(|to_multiply| {
                                Value::saturating_from_num(
                                    to_multiply.az::<f32>() * ctx.param().to_u().az::<f32>(),
                                )
                            })
                            .collect::<Vec<_>>()
                    }
                })
                .unwrap_or(vec![])
                .into_iter(),
        )
    }
}
