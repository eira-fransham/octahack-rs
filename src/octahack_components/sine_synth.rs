use crate::{AnyIter, Component, GetInput, GetOutput, GetParam, Value, ValueIter};

// TODO: This should be in the context
const FREQUENCY: usize = 44100;

crate::specs! {
    mod synth {
        IO: Value
    }
}

pub use self::synth::Specifier;
use self::synth::IO;

impl Default for self::synth::Params {
    fn default() -> Self {
        self::synth::Params { IO: freq(440) }
    }
}

#[derive(Default)]
// TODO: Make the wave configurable
pub struct SineSynth {
    tick: f64,
}

impl SineSynth {
    pub fn new() -> Self {
        Self::default()
    }
}

fn volt_to_octave(volts: Value) -> f64 {
    440.0f64 * (f64::from(volts) / f64::from(super::VOLT)).exp2()
}

// 440 * (2 ^ 10x) = freq
// log2(freq / 440) / 10. = x

pub fn freq(freq: impl Into<f64>) -> Value {
    let freq = freq.into();
    Value::saturating_from_num((freq / 440.).log2() * f64::from(super::VOLT))
}

type Iter = impl ValueIter + Send;

impl Component for SineSynth {
    type InputSpecifier = !;
    type OutputSpecifier = Specifier;
    type ParamSpecifier = Specifier;
    type OutputIter = Iter;

    fn update<Ctx>(&self, ctx: &Ctx) -> Self
    where
        Ctx: GetInput<<Self as Component>::InputSpecifier>
            + GetParam<<Self as Component>::ParamSpecifier>,
    {
        let freq = volt_to_octave(ctx.param::<IO>());
        SineSynth {
            tick: self.tick + freq / FREQUENCY as f64,
        }
    }
}

impl GetOutput<self::synth::IO> for SineSynth {
    fn output<Ctx>(&self, _: &Ctx) -> Iter
    where
        Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>,
    {
        AnyIter::from(std::iter::once(Value::saturating_from_num(
            (2. * std::f64::consts::PI * self.tick).sin() / 2.,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::{super::VOLT, volt_to_octave, Value};

    #[test]
    fn test_volt_to_freq() {
        assert_eq!(volt_to_octave(Value::from_num(0.0)) as u32, 440);
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

        for i in 1..440 * 12 {
            assert_eq!(volt_to_octave(super::freq(i)).round() as u32, i);
        }
    }
}
