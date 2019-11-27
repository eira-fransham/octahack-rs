use crate::{
    components::NoIter,
    context::GetGlobalInput,
    params::{HasStorage, ParamStorage, Possibly},
    rack::{self, InternalWire},
    AnyComponent, AnyIter, Rack, RuntimeSpecifier, SpecId, Value, ValueIter, ValueKind,
};
use fixed::types::I1F15;
use nom_midi::MidiEventType;
use rodio::Source;
use std::borrow::Cow;

fn num_audio_channels<Spec>() -> u8
where
    Spec: RuntimeSpecifier,
{
    let mut out = 0;

    for &ty in Spec::TYPES {
        if ty.kind == ValueKind::Continuous {
            out += ty.channels.unwrap();
        }
    }

    out
}

struct OrZero<I> {
    iter: Option<I>,
    min_len: usize,
}

impl<I> Iterator for OrZero<I>
where
    I: std::iter::ExactSizeIterator<Item = i16>,
{
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(val) = self.iter.as_mut().and_then(|i| i.next()) {
            self.min_len -= 1;
            Some(val)
        } else if self.min_len > 0 {
            self.min_len -= 1;
            Some(0)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<I> std::iter::ExactSizeIterator for OrZero<I>
where
    I: std::iter::ExactSizeIterator<Item = i16>,
{
    fn len(&self) -> usize {
        self.iter
            .as_ref()
            .map(|i| i.len())
            .unwrap_or(0)
            .max(self.min_len)
    }
}

pub struct AudioStreamer<S, C, InputSpec, OutputSpec>
where
    C: AnyComponent,
    OutputSpec: HasStorage<InternalWire>,
{
    output_id: SpecId,
    out_iter: Option<Box<dyn Iterator<Item = i16> + Send>>,
    sample_rate: u32,
    audio_inputs: S,
    rack: Rack<C, InputSpec, OutputSpec>,
}

impl<S, C, InputSpec, OutputSpec>
    AudioStreamer<rodio::source::UniformSourceIterator<S, i16>, C, InputSpec, OutputSpec>
where
    C: AnyComponent,
    InputSpec: RuntimeSpecifier,
    OutputSpec: RuntimeSpecifier + HasStorage<InternalWire>,
    S: Source + Iterator + 'static,
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
                num_audio_channels::<InputSpec>() as u16,
                sample_rate,
            ),
        )
    }
}

const DEFAULT_SAMPLE_RATE: u32 = 44100;

impl<S, C, InputSpec, OutputSpec> AudioStreamer<S, C, InputSpec, OutputSpec>
where
    C: AnyComponent,
    S: Source + Iterator<Item = i16> + 'static,
    InputSpec: RuntimeSpecifier,
    OutputSpec: RuntimeSpecifier + HasStorage<InternalWire>,
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
            out_iter: None,
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
            && source.channels() == num_audio_channels::<OutputSpec>() as u16
        {
            Some(Self::new_unchecked(sample_rate, rack, source))
        } else {
            None
        }
    }
}

pub struct Context<'a> {
    sources: Cow<'a, [i16]>,
}

impl<'a, InputSpec> GetGlobalInput<InputSpec> for Context<'a>
where
    InputSpec: RuntimeSpecifier,
{
    type Iter = AnyIter<NoIter<MidiEventType>, std::vec::IntoIter<Value>>;

    // `None` means that this input is not wired
    fn input(&self, spec: InputSpec) -> Option<Self::Iter> {
        let mut id = 0;

        for i in 0..spec.id() {
            if InputSpec::from_id(i).value_type().kind == ValueKind::Continuous {
                id += InputSpec::TYPES[i].channels.unwrap();
            }
        }

        Some(AnyIter::from(
            // TODO
            self.sources
                [id as usize..(id + InputSpec::TYPES[spec.id()].channels.unwrap()) as usize]
                .iter()
                .map(|&val| Value::from_num(I1F15::from_bits(val)))
                .collect::<Vec<_>>()
                .into_iter(),
        ))
    }
}

impl<S, C, InputSpec, OutputSpec> AudioStreamer<S, C, InputSpec, OutputSpec>
where
    S: Source + Iterator<Item = i16> + 'static,
    C: AnyComponent + 'static,
    InputSpec: RuntimeSpecifier,
    OutputSpec: RuntimeSpecifier + HasStorage<InternalWire>,
{
    fn update(&mut self) -> Option<impl ExactSizeIterator<Item = i16>> {
        // Originally this was done with a
        loop {
            let mut sources = vec![];
            if self.output_id == 0 {
                // `debug` because we should assert this in `fn new`
                debug_assert_eq!(self.audio_inputs.sample_rate(), self.sample_rate());
                debug_assert_eq!(
                    self.audio_inputs.channels(),
                    num_audio_channels::<InputSpec>() as u16
                );
                for _ in 0..self.audio_inputs.channels() {
                    sources.push(self.audio_inputs.next()?);
                }

                let sources = Cow::Borrowed(&sources[..]);
                let ctx = Context { sources };

                self.rack.update::<Context>(&ctx);
            }

            let new_id = {
                let mut id = self.output_id;
                loop {
                    if let Some(&ty) = OutputSpec::TYPES.get(id) {
                        if ty.kind == ValueKind::Continuous {
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
                let sources = Cow::Owned(sources);
                let ctx: Context<'static> = Context { sources };

                self.output_id = new_id + 1;

                return Some(OrZero {
                    iter: self
                        .rack
                        .output(OutputSpec::VALUES[new_id].clone(), &ctx)
                        .map(|val| {
                            val.analog()
                                .unwrap()
                                .map(|val| I1F15::from_num(val).to_bits())
                        }),
                    min_len: OutputSpec::from_id(new_id).value_type().channels.unwrap() as usize,
                });
            } else {
                self.output_id = 0;
            }
        }
    }
}

impl<S, C, InputSpec, OutputSpec> Iterator for AudioStreamer<S, C, InputSpec, OutputSpec>
where
    S: Source + Iterator<Item = i16> + 'static,
    C: AnyComponent + 'static,
    InputSpec: RuntimeSpecifier,
    OutputSpec: RuntimeSpecifier + HasStorage<InternalWire>,
{
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(a) = self.out_iter.as_mut().and_then(|i| i.next()) {
            return Some(a);
        }

        let mut new_iter = self.update()?;
        let out = new_iter.next();
        if new_iter.len() > 0 {
            self.out_iter = Some(Box::new(new_iter));
        }
        out
    }
}

impl<S, C, InputSpec, OutputSpec> rodio::Source for AudioStreamer<S, C, InputSpec, OutputSpec>
where
    S: Source + Iterator<Item = i16> + 'static,
    C: AnyComponent + 'static,
    InputSpec: RuntimeSpecifier,
    OutputSpec: RuntimeSpecifier + HasStorage<InternalWire>,
{
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u16 {
        num_audio_channels::<OutputSpec>() as u16
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}
