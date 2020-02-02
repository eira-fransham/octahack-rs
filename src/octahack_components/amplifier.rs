use crate::{Component, Context, GetOutput, UiElement, Value, ValueExt};
use az::Az;

crate::specs! {
    mod amplifier {
        Only: crate::Value
    }
}

use amplifier::Only;
pub use amplifier::Specifier;

impl Default for amplifier::Params {
    fn default() -> Self {
        Self {
            Only: Default::default(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct Amplifier;

impl UiElement for Amplifier {
    const NAME: &'static str = "Amplifier";
}

impl Component for Amplifier {
    type InputSpecifier = Specifier;
    type OutputSpecifier = Specifier;
    type ParamSpecifier = Specifier;

    fn update<Ctx>(&self, _: &Ctx) -> Self
    where
        Ctx: Context<Self>,
    {
        *self
    }
}

impl GetOutput<amplifier::Only> for Amplifier {
    type Iter = impl ExactSizeIterator<Item = Value> + Send;

    fn output<Ctx>(&self, ctx: &Ctx) -> Self::Iter
    where
        Ctx: Context<Self>,
    {
        let inputs = if let Some(inputs) = ctx.input::<Only>() {
            inputs
        } else {
            return Vec::<Value>::new().into_iter().into();
        };

        inputs
            .map(|to_multiply| {
                Value::saturating_from_num(to_multiply.az::<f32>() * ctx.param().to_u().az::<f32>())
            })
            .collect::<Vec<_>>()
            .into_iter()
    }
}
