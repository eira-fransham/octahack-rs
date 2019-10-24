use crate::{ComponentSet, Continuous, Continuous16, Rack, SpecId, Specifier, Value, ValueType};
use rodio::Source;
use std::{collections::HashMap, hash::Hash, iter, slice, time};

fn num_audio_channels<OutputSpec>() -> u16
where
    OutputSpec: Specifier,
{
    let mut out = 0;

    for &ty in OutputSpec::TYPES {
        if ty == ValueType::Continuous {
            out += 1;
        }
    }

    out
}

pub struct AudioStreamer<S, C, InputSpec, OutputSpec> {
    output_id: SpecId,
    cur_samples: Vec<Option<i16>>,
    sample_rate: u32,
    audio_inputs: S,
    rack: Rack<C, InputSpec, OutputSpec>,
}

impl<S, C, InputSpec, OutputSpec>
    AudioStreamer<rodio::source::UniformSourceIterator<S, i16>, C, InputSpec, OutputSpec>
where
    InputSpec: Specifier,
    OutputSpec: Specifier,
    S: Source + Iterator,
    S::Item: rodio::Sample,
{
    pub fn new_convert(
        sample_rate: impl Into<Option<u32>>,
        rack: Rack<C, InputSpec, OutputSpec>,
        source: S,
    ) -> Self {
        let sample_rate = sample_rate.into().unwrap_or(DEFAULT_SAMPLE_RATE);
        Self::new_unchecked(
            sample_rate,
            rack,
            rodio::source::UniformSourceIterator::new(
                source,
                num_audio_channels::<OutputSpec>(),
                sample_rate,
            ),
        )
    }
}

const DEFAULT_SAMPLE_RATE: u32 = 44100;

impl<S, C, InputSpec, OutputSpec> AudioStreamer<S, C, InputSpec, OutputSpec>
where
    S: Source + Iterator<Item = i16>,
    InputSpec: Specifier,
    OutputSpec: Specifier,
{
    pub fn new_unchecked(
        sample_rate: impl Into<Option<u32>>,
        rack: Rack<C, InputSpec, OutputSpec>,
        source: S,
    ) -> Self {
        AudioStreamer {
            output_id: 0,
            rack,
            sample_rate: sample_rate.into().unwrap_or(DEFAULT_SAMPLE_RATE),
            cur_samples: vec![None; InputSpec::VALUES.len()],
            audio_inputs: source,
        }
    }

    pub fn new(
        sample_rate: impl Into<Option<u32>>,
        rack: Rack<C, InputSpec, OutputSpec>,
        source: S,
    ) -> Option<Self> {
        let sample_rate = sample_rate.into().unwrap_or(DEFAULT_SAMPLE_RATE);
        if source.sample_rate() == sample_rate
            && source.channels() == num_audio_channels::<OutputSpec>()
        {
            Some(Self::new_unchecked(sample_rate, rack, source))
        } else {
            None
        }
    }
}

impl<S, C, InputSpec, OutputSpec> Iterator for AudioStreamer<S, C, InputSpec, OutputSpec>
where
    S: Source + Iterator<Item = i16>,
    C: ComponentSet,
    InputSpec: Specifier,
    OutputSpec: Specifier,
{
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        macro_rules! context {
            ($sources:expr) => {
                &|input: InputSpec| {
                    $sources[input.id()].map(|val| {
                        Value::Continuous(Continuous::from_num(Continuous16::from_bits(val)))
                    })
                }
            };
        }

        if self.output_id == 0 {
            // `debug` because we should assert this in `fn new`
            debug_assert_eq!(self.audio_inputs.sample_rate(), self.sample_rate());
            debug_assert_eq!(
                self.audio_inputs.channels(),
                num_audio_channels::<OutputSpec>()
            );
            for val in &mut self.cur_samples {
                *val = self.audio_inputs.next();
            }

            let sources = &self.cur_samples;
            let ctx = context!(sources);

            self.rack.update(ctx);
        }

        let new_id = {
            let mut id = self.output_id;
            loop {
                if let Some(&ty) = OutputSpec::TYPES.get(id) {
                    if ty == ValueType::Continuous {
                        break Some(id);
                    } else {
                        id += 1;
                        continue;
                    }
                } else {
                    break None;
                }
            }
        };

        if let Some(new_id) = new_id {
            let sources = &self.cur_samples;
            let ctx = context!(sources);

            self.output_id = new_id + 1;
            let out = self
                .rack
                .output(OutputSpec::VALUES[new_id].clone(), ctx)
                .map(|val| val.continuous().unwrap())
                .unwrap_or_default();

            Some(Continuous16::from_num(out).to_bits())
        } else {
            self.output_id = 0;
            self.next()
        }
    }
}

impl<S, C, InputSpec, OutputSpec> rodio::Source for AudioStreamer<S, C, InputSpec, OutputSpec>
where
    S: Source + Iterator<Item = i16>,
    C: ComponentSet,
    InputSpec: Specifier,
    OutputSpec: Specifier,
{
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u16 {
        num_audio_channels::<OutputSpec>()
    }

    fn total_duration(&self) -> Option<time::Duration> {
        None
    }
}
