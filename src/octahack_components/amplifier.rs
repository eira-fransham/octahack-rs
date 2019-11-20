use crate::{
    AnyIter, Component, GetInput, GetParam, Param, SpecId, Specifier, Value, ValueExt, ValueIter,
    ValueType,
};
use az::Az;

#[derive(Copy, Clone)]
pub struct AmplifierIO(pub u8);

const AMPLIFIER_IO_COUNT: usize = 8;

impl Specifier for AmplifierIO {
    const VALUES: &'static [Self] = &[
        AmplifierIO(0),
        AmplifierIO(1),
        AmplifierIO(2),
        AmplifierIO(3),
        AmplifierIO(4),
        AmplifierIO(5),
        AmplifierIO(6),
        AmplifierIO(7),
    ];
    const TYPES: &'static [ValueType] = &[ValueType::continuous(); AMPLIFIER_IO_COUNT];

    fn id(&self) -> SpecId {
        self.0 as _
    }

    fn from_id(id: SpecId) -> Self {
        assert!(id < AMPLIFIER_IO_COUNT);
        AmplifierIO(id as _)
    }
}

impl Param for AmplifierIO {
    fn default(&self) -> Value {
        Value::default()
    }
}

#[derive(Copy, Clone)]
pub struct Amplifier;

impl Component for Amplifier {
    type InputSpecifier = AmplifierIO;
    type OutputSpecifier = AmplifierIO;
    type ParamSpecifier = AmplifierIO;
    type OutputIter = impl ValueIter + Send;

    fn output<Ctx>(&self, id: Self::OutputSpecifier, ctx: Ctx) -> Self::OutputIter
    where
        Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>,
    {
        

        AnyIter::from(
            ctx.input(id)
                .map(|inputs| {
                    {
                        inputs
                            .analog()
                            .unwrap()
                            .map(|to_multiply| {
                                Value::saturating_from_num(
                                    to_multiply.az::<f32>() * ctx.param(id).to_u().az::<f32>(),
                                )
                            })
                            .collect::<Vec<_>>()
                    }
                })
                .unwrap_or(vec![])
                .into_iter(),
        )
    }

    fn update<Ctx>(&self, _: Ctx) -> Self
    where
        Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>,
    {
        *self
    }
}
