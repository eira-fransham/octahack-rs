use crate::{Component, Context, DisplayParam, GetOutput, UiElement, Value};
use std::fmt;

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

impl DisplayParam for params::Freq {
    type Display = impl fmt::Display;

    fn display(val: Value) -> Self::Display {
        struct FreqDisplay(Value);

        impl fmt::Display for FreqDisplay {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    f,
                    "{}Hz",
                    ((volt_to_octave(self.0) * 100.) as u64) as f64 / 100.
                )
            }
        }

        FreqDisplay(val)
    }
}

impl Default for params::Params {
    fn default() -> Self {
        params::Params { Freq: freq(440) }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Synth {
    tick: f64,
}

impl UiElement for Synth {
    const NAME: &'static str = "Synth";
}

impl Synth {
    pub fn new() -> Self {
        Self::default()
    }
}

fn volt_to_octave(volts: impl Into<Value>) -> f64 {
    440. * volts.into().exp2()
}

// 440 * (2 ^ x) = freq
// log2(freq / 440) = x

// This converts a frequency in Hz to a number of virtual "volts"
pub fn freq(freq: impl Into<Value>) -> Value {
    (freq.into() / 440.).log2()
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
            tick: ((self.tick + freq / ctx.sample_rate() as f64) % 1.),
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

        iter::once((2. * f64::consts::PI * self.tick).sin())
    }
}

impl GetOutput<output::Saw> for Synth {
    type Iter = impl ExactSizeIterator<Item = Value> + Send;

    fn output<Ctx>(&self, _: &Ctx) -> Self::Iter
    where
        Ctx: Context<Self>,
    {
        use std::iter;

        iter::once(1. - 2. * self.tick)
    }
}

impl GetOutput<output::Square> for Synth {
    type Iter = impl ExactSizeIterator<Item = Value> + Send;

    fn output<Ctx>(&self, _: &Ctx) -> Self::Iter
    where
        Ctx: Context<Self>,
    {
        use std::iter;

        iter::once(if self.tick < 0.5 { 1. } else { -1. })
    }
}

#[cfg(test)]
mod tests {
    use super::{volt_to_octave, Value};

    #[test]
    fn test_volt_to_freq() {
        assert_eq!(volt_to_octave(0) as u32, 440);
        assert_eq!(volt_to_octave(1) as u32, 880);
        assert_eq!(volt_to_octave(2) as u32, 1760);

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
