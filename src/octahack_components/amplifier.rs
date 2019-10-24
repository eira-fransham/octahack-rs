use crate::{Component, GetInput, GetParam, Param, SpecId, Specifier, Value, ValueType};

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
    const TYPES: &'static [ValueType] = &[ValueType::Continuous; AMPLIFIER_IO_COUNT];

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
        Value::Continuous(crate::Continuous::default())
    }
}

pub struct Amplifier;

impl Component for Amplifier {
    type InputSpecifier = AmplifierIO;
    type OutputSpecifier = AmplifierIO;
    type ParamSpecifier = AmplifierIO;

    fn output<Ctx>(&self, id: Self::OutputSpecifier, ctx: &mut Ctx) -> Option<Value>
    where
        for<'a> &'a mut Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>,
    {
        use az::Cast;

        let to_multiply: f32 = ctx.input(id)?.continuous().unwrap().cast();
        // TODO: Return `Result<Option<Value>, SomeError>` so we can differentiate between
        //       "no value" and "an error happened"
        let multiplication_factor: f32 =
            fixed::FixedU32::<typenum::consts::U32>::from_num(ctx.param(id).continuous().unwrap())
                .cast();

        Some(Value::from(
            to_multiply * 2. * (multiplication_factor + 0.5),
        ))
    }
}
