use crate::{
    components::Update, AnyIter, Component, GetInput, GetOutput, GetParam, Value, ValueExt,
    ValueIter,
};
use az::Az;

crate::specs! {
    mod amplifier {
        Only: Value
    }
}

pub use self::amplifier::Specifier;
use self::amplifier::IO;

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
}

impl<Ctx> Update<Ctx> for Amplifier {
    fn update(&self, _: Ctx) -> Self {
        *self
    }
}

impl<Ctx> GetOutput<Ctx, self::amplifier::Only> for Amplifier
where
    Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::OutputSpecifier, IO>,
{
    fn output(&self, ctx: Ctx) -> OutputIter {
        AnyIter::from(
            ctx.input(Specifier::Only)
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
