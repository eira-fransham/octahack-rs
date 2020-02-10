use crate::{context::ContextMetaExt, Component, Context, GetOutput, Value};
use az::Az;
use std::time::Duration;

crate::specs! {
    mod params {
        File: Option<crate::context::FileId<crate::Value>>,
        Speed: crate::Value
    }

    mod output {
        Output: crate::Value
    }
}

impl Default for params::Params {
    fn default() -> Self {
        Self {
            File: None,
            Speed: 1.,
        }
    }
}

#[derive(Copy, Clone)]
pub struct FilePlayer {
    seek_pos: Duration,
}

impl Component for FilePlayer {
    type InputSpecifier = !;
    type OutputSpecifier = output::Specifier;
    type ParamSpecifier = params::Specifier;

    fn update<Ctx>(&self, ctx: &Ctx) -> Self
    where
        Ctx: Context<Self>,
    {
        Self {
            seek_pos: self.seek_pos + ctx.sample_duration(),
        }
    }
}

impl GetOutput<output::Output> for FilePlayer {
    type Iter = impl ExactSizeIterator<Item = Value> + Send;

    fn output<Ctx>(&self, ctx: &Ctx) -> Self::Iter
    where
        Ctx: Context<Self>,
    {
        use crate::context::File;

        if let Some(file) = ctx.param::<params::File>() {
            ctx.read(file).at(self.seek_pos).collect::<Vec<_>>()
        } else {
            std::iter::empty().collect::<Vec<_>>()
        }
        .into_iter()
    }
}
