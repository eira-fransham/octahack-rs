use crate::{
    AnyIter, Component, GetInput, GetOutput, GetParam, Param, SpecId, Specifier, Value, ValueIter,
    ValueType,
};
use fixed::types::I0F32;

// TODO: This should be in the context
const FREQUENCY: usize = 44100;

#[derive(Copy, Clone)]
pub struct SynthIO;

impl Specifier for SynthIO {
    const VALUES: &'static [Self] = &[SynthIO];
    const TYPES: &'static [ValueType] = &[ValueType::continuous()];

    fn id(&self) -> SpecId {
        0
    }

    fn from_id(id: SpecId) -> Self {
        assert_eq!(id, 0);
        SynthIO
    }
}

impl Param for SynthIO {
    fn default(&self) -> Value {
        freq(440)
    }
}

// TODO: Make the wave configurable
pub struct SineSynth {
    tick: f64,
}

impl SineSynth {
    pub fn new() -> Self {
        SineSynth { tick: 0. }
    }
}

fn volt_to_octave(volts: I0F32) -> f64 {
    440.0f64 * (f64::from(volts) / f64::from(super::VOLT)).exp2()
}

// 440 * (2 ^ 10x) = freq
// log2(freq / 440) / 10. = x

pub fn freq(freq: impl Into<f64>) -> I0F32 {
    let freq = freq.into();
    I0F32::saturating_from_num((freq / 440.).log2() * f64::from(super::VOLT))
}

impl Component for SineSynth {
    type InputSpecifier = !;
    type OutputSpecifier = SynthIO;
    type ParamSpecifier = SynthIO;
}

impl GetOutput for SineSynth {
    type OutputIter = impl ValueIter + Send;

    fn update<Ctx>(&mut self, ctx: Ctx)
    where
        Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>,
    {
        let freq = volt_to_octave(ctx.param(SynthIO));
        self.tick += freq / FREQUENCY as f64;
    }

    fn output<Ctx>(&self, _: Self::OutputSpecifier, _: Ctx) -> Self::OutputIter
    where
        Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>,
    {
        AnyIter::from(std::iter::once(I0F32::saturating_from_num(
            (2. * std::f64::consts::PI * self.tick).sin() / 2.,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::{super::VOLT, volt_to_octave, I0F32};

    #[test]
    fn test_volt_to_freq() {
        assert_eq!(volt_to_octave(I0F32::from_num(0.0)) as u32, 440);
        assert_eq!(volt_to_octave(VOLT) as u32, 880);
        assert_eq!(volt_to_octave(VOLT * 2) as u32, 1760);

        let points = 1000;
        for i in 1..points {
            let actual_freq = i as f64 / points as f64;
            assert_eq!(
                (volt_to_octave(super::freq(actual_freq)) * 1000.).round() as u32,
                (actual_freq * 1000.).round() as u32
            );
        }
        for i in 1..880 {
            assert_eq!(volt_to_octave(super::freq(i)).round() as u32, i);
        }
    }
}
