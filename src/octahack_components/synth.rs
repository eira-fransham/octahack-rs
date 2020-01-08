use crate::{Component, Context, GetOutput, Value};
use fast_floats::FF64;

crate::specs! {
    pub mod params {
        Freq: crate::Value
    }

    pub mod output {
        Sine: crate::Value,
        Saw: crate::Value,
        Square: crate::Value
    }
}

impl Default for params::Params {
    fn default() -> Self {
        params::Params { Freq: freq(440) }
    }
}

#[derive(Default)]
pub struct Synth {
    tick: f64,
}

impl Synth {
    pub fn new() -> Self {
        Self::default()
    }
}

fn f(f: f64) -> FF64 {
    FF64::from(f)
}

fn volt_to_octave(volts: Value) -> f64 {
    (f(440.0f64) * f((f(f64::from(volts)) / f(f64::from(super::VOLT))).0.exp2())).0
}

// 440 * (2 ^ 10x) = freq
// log2(freq / 440) / 10. = x

// This converts a frequency in Hz to a number of virtual "volts"
pub fn freq(freq: impl Into<f64>) -> Value {
    let freq = freq.into();
    Value::saturating_from_num((f((f(freq) / f(440.)).0.log2()) * f(f64::from(super::VOLT))).0)
}

impl Component for Synth {
    type InputSpecifier = !;
    type OutputSpecifier = output::Specifier;
    type ParamSpecifier = params::Specifier;

    fn update<Ctx>(&self, ctx: &Ctx) -> Self
    where
        Ctx: Context<Self>,
    {
        let freq = volt_to_octave(ctx.param::<params::Freq>());

        Synth {
            tick: ((f(self.tick) + f(freq) / f(ctx.sample_rate() as f64)) % f(1.)).0,
        }
    }
}

impl GetOutput<output::Sine> for Synth {
    type Iter = impl ExactSizeIterator<Item = Value> + Send;

    fn output<Ctx>(&self, _: &Ctx) -> Self::Iter
    where
        Ctx: Context<Self>,
    {
        use std::{f64, iter};

        iter::once(Value::saturating_from_num(
            (f(2.) * f(f64::consts::PI) * self.tick).0.sin(),
        ))
    }
}

impl GetOutput<output::Saw> for Synth {
    type Iter = impl ExactSizeIterator<Item = Value> + Send;

    fn output<Ctx>(&self, _: &Ctx) -> Self::Iter
    where
        Ctx: Context<Self>,
    {
        use std::iter;

        iter::once(Value::saturating_from_num((f(1.) - f(2.) * self.tick).0))
    }
}

impl GetOutput<output::Square> for Synth {
    type Iter = impl ExactSizeIterator<Item = Value> + Send;

    fn output<Ctx>(&self, _: &Ctx) -> Self::Iter
    where
        Ctx: Context<Self>,
    {
        use std::iter;

        iter::once(Value::saturating_from_num(if self.tick < 0.5 {
            1.
        } else {
            -1.
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{super::VOLT, f, volt_to_octave, Value};

    #[test]
    fn test_volt_to_freq() {
        assert_eq!(volt_to_octave(Value::from_num(0.0)) as u32, 440);
        assert_eq!(volt_to_octave(VOLT) as u32, 880);
        assert_eq!(volt_to_octave(VOLT * 2) as u32, 1760);

        let points = 1000;
        for i in 1..points {
            let actual_freq = f(i as f64) / f(points as f64);
            assert_eq!(
                (volt_to_octave(super::freq(actual_freq.0)) * 1000.).round() as u32,
                (actual_freq * f(1000.)).0.round() as u32
            );
        }

        for i in 1..440 * 12 {
            assert_eq!(volt_to_octave(super::freq(i)).round() as u32, i);
        }
    }
}
