use crate::{
    AnyIter, Component, GetInput, GetOutput, GetParam, Param, SpecId, Specifier, Value, ValueIter,
    ValueType,
};
use fixed::types::I0F32;

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
        I0F32::default()
    }
}

pub struct Amplifier;

impl Component for Amplifier {
    type InputSpecifier = AmplifierIO;
    type OutputSpecifier = AmplifierIO;
    type ParamSpecifier = AmplifierIO;
}

impl GetOutput for Amplifier {
    type OutputIter = impl ValueIter + Send;

    fn output<Ctx>(&self, id: Self::OutputSpecifier, ctx: Ctx) -> Self::OutputIter
    where
        Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>,
    {
        use az::Cast;

        AnyIter::from(
            ctx.input(id)
                .map(|inputs| {
                    {
                        inputs
                            .analog()
                            .unwrap()
                            .map(|i| i.cast())
                            .map(|to_multiply: f32| {
                                // TODO: Return `Result<Option<Value>, SomeError>` so we can differentiate between
                                //       "no value" and "an error happened"
                                let multiplication_factor: f32 =
                                    fixed::FixedU32::<typenum::consts::U32>::from_num(
                                        ctx.param(id),
                                    )
                                    .cast();

                                Value::saturating_from_num(
                                    to_multiply * 2. * (multiplication_factor + 0.5),
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
