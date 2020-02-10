use crate::{Component, Context, DisplayParam, GetOutput, UiElement, Value};
use az::Az;
use std::fmt;

crate::specs! {
    pub mod params {
        Amount: crate::Value
    }

    pub mod input {
        Input: crate::Value
    }

    pub mod output {
        Output: crate::Value
    }
}

impl DisplayParam for params::Amount {
    type Display = impl fmt::Display;

    fn display(val: Value) -> Self::Display {
        struct PercDisplay(Value);

        impl fmt::Display for PercDisplay {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    f,
                    "{}%",
                    ((f64::from(self.0) * 100.0 + 100.0).round() as i32)
                )
            }
        }

        PercDisplay(val)
    }
}

impl Default for params::Params {
    fn default() -> Self {
        Self {
            Amount: Default::default(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Amplifier;

impl UiElement for Amplifier {
    const NAME: &'static str = "Amplifier";
}

impl Component for Amplifier {
    type InputSpecifier = input::Specifier;
    type OutputSpecifier = output::Specifier;
    type ParamSpecifier = params::Specifier;

    fn update<Ctx>(&self, _: &Ctx) -> Self
    where
        Ctx: Context<Self>,
    {
        *self
    }
}

impl GetOutput<output::Output> for Amplifier {
    type Iter = impl ExactSizeIterator<Item = Value> + Send;

    fn output<Ctx>(&self, ctx: &Ctx) -> Self::Iter
    where
        Ctx: Context<Self>,
    {
        let inputs = if let Some(inputs) = ctx.input::<input::Input>() {
            inputs
        } else {
            return Vec::<Value>::new().into_iter().into();
        };

        let amount = ctx.param();
        inputs
            .map(|to_multiply| to_multiply * amount)
            .collect::<Vec<_>>()
            .into_iter()
    }
}
