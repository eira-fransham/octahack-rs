use crate::{Value, ValueIter};

pub struct QuickContext<C, InputFn, ParamFn> {
    ctx: C,
    input_fn: InputFn,
    param_fn: ParamFn,
}

impl<InputFn> QuickContext<(), InputFn, ()> {
    pub fn input(input_fn: InputFn) -> Self {
        Self::new((), input_fn, ())
    }
}

impl<C, InputFn, ParamFn> QuickContext<C, InputFn, ParamFn> {
    pub fn new(ctx: C, input_fn: InputFn, param_fn: ParamFn) -> Self {
        QuickContext {
            ctx,
            input_fn,
            param_fn,
        }
    }
}

// TODO: Support MIDI inputs
impl<C, InputFn, ParamFn, Spec, I> GetInput<Spec> for QuickContext<C, InputFn, ParamFn>
where
    InputFn: Fn(&C, Spec) -> Option<I>,
    I: ValueIter + Send,
{
    type Iter = I;

    fn input(&self, spec: Spec) -> Option<Self::Iter> {
        (self.input_fn)(&self.ctx, spec)
    }
}

impl<C, InputFn, ParamFn, Spec> GetParam<Spec> for QuickContext<C, InputFn, ParamFn>
where
    ParamFn: Fn(&C, Spec) -> Value,
{
    fn param(&self, spec: Spec) -> Value {
        (self.param_fn)(&self.ctx, spec)
    }
}

impl<C, Spec> GetInput<Spec> for &'_ C
where
    C: GetInput<Spec>,
{
    type Iter = C::Iter;

    fn input(&self, spec: Spec) -> Option<Self::Iter> {
        C::input(*self, spec)
    }
}

impl<C, Spec> GetParam<Spec> for &'_ C
where
    C: GetParam<Spec>,
{
    fn param(&self, spec: Spec) -> Value {
        C::param(*self, spec)
    }
}

pub trait Context<ISpec, PSpec>: GetInput<ISpec> + GetParam<PSpec> {
    /// Samples per second
    fn samples(&self) -> usize;
}

pub trait GetInput<Spec> {
    type Iter: ValueIter + Send;

    // `None` means that this input is not wired
    fn input(&self, spec: Spec) -> Option<Self::Iter>;
}

pub trait GetParam<Spec> {
    fn param(&self, spec: Spec) -> Value;
}
