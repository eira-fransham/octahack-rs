use crate::{
    components::{EnumerateValues, PossiblyIter, ValueIterImplHelper},
    context::{ContextMeta, GetFunctionParam},
    params::HasStorage,
    rack::InternalWire,
    AnyComponent, Rack, RuntimeSpecifier, SpecId, Value, ValueKind,
};
use fixed::types::I1F15;
use rodio::Source;
use std::{borrow::Cow, marker::PhantomData};

trait Sources<'a> {
    type Iter;

    fn total_channels<T>(&'a self) -> Option<usize>
    where
        Self::Iter: PossiblyIter<T>;

    fn next(&'a mut self) -> Self::Iter;
}

fn num_audio_channels<Spec>() -> u8
where
    Spec: EnumerateValues,
{
    let mut out = 0;

    for v in Spec::values() {
        let ty = v.value_type();

        if ty.kind == ValueKind::Continuous {
            out += ty.channels.unwrap().get();
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
    out_iter: Option<OutputIter<S, C, InputSpec, OutputSpec>>,
    sample_rate: u32,
    audio_inputs: S,
    rack: Rack<C, InputSpec, OutputSpec>,
}

impl<S, C, InputSpec, OutputSpec>
    AudioStreamer<rodio::source::UniformSourceIterator<S, i16>, C, InputSpec, OutputSpec>
where
    C: AnyComponent,
    InputSpec: EnumerateValues,
    OutputSpec: EnumerateValues + HasStorage<InternalWire>,
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
    OutputSpec: EnumerateValues + HasStorage<InternalWire>,
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

pub struct Context<'a, ISpec> {
    sources: Cow<'a, [i16]>,
    sample_rate: u32,
    _marker: PhantomData<ISpec>,
}

impl<'a, I> ContextMeta for Context<'a, I> {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl<'a, InputSpec> GetFunctionParam for Context<'a, InputSpec>
where
    InputSpec: RuntimeSpecifier,
{
    type InputSpec = InputSpec;
    type Iter = <Value as ValueIterImplHelper<std::vec::IntoIter<Value>>>::AnyIter;

    // `None` means that this input is not wired
    fn input(&self, spec: InputSpec) -> Option<Self::Iter> {
        let mut id = 0;

        for i in 0..spec.id() {
            let ty = InputSpec::from_id(i).value_type();
            if ty.kind == ValueKind::Continuous {
                id += ty.channels.unwrap().get();
            }
        }

        Some(
            // TODO
            self.sources[id as usize
                ..(id
                    + InputSpec::from_id(id as _)
                        .value_type()
                        .channels
                        .unwrap()
                        .get()) as usize]
                .iter()
                .map(|&val| Value::from_num(I1F15::from_bits(val)))
                .collect::<Vec<_>>()
                .into_iter()
                .into(),
        )
    }
}

type OutputIter<S, C, InputSpec, OutputSpec> = impl ExactSizeIterator<Item = i16>;

impl<S, C, InputSpec, OutputSpec> AudioStreamer<S, C, InputSpec, OutputSpec>
where
    S: Source + Iterator<Item = i16> + 'static,
    C: AnyComponent + 'static,
    InputSpec: EnumerateValues,
    OutputSpec: EnumerateValues + HasStorage<InternalWire>,
{
    fn update(&mut self) -> Option<OutputIter<S, C, InputSpec, OutputSpec>> {
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
                let ctx = Context {
                    sample_rate: self.sample_rate(),
                    sources,
                    _marker: PhantomData,
                };

                self.rack.update::<Context<InputSpec>>(&ctx);
            }

            let new_id = {
                let mut id = self.output_id;
                loop {
                    if let Some(v) = OutputSpec::values().nth(id) {
                        if v.value_type().kind == ValueKind::Continuous {
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
                let ctx: Context<'static, InputSpec> = Context {
                    sample_rate: self.sample_rate(),
                    sources,
                    _marker: PhantomData,
                };

                self.output_id = new_id + 1;

                return Some(OrZero {
                    iter: self
                        .rack
                        .output(OutputSpec::from_id(new_id), &ctx)
                        .map(|iter| {
                            PossiblyIter::<Value>::try_iter(iter)
                                .unwrap_or_else(|_| unimplemented!())
                                .map(|val| I1F15::from_num(val).to_bits())
                        }),
                    min_len: OutputSpec::from_id(new_id)
                        .value_type()
                        .channels
                        .unwrap()
                        .get() as usize,
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
    InputSpec: EnumerateValues,
    OutputSpec: EnumerateValues + HasStorage<InternalWire>,
{
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(a) = self.out_iter.as_mut().and_then(|i| i.next()) {
            return Some(a);
        }

        let mut new_iter = self.update()?;
        let out = new_iter.next();
        if new_iter.len() > 0 {
            self.out_iter = Some(new_iter);
        }
        out
    }
}

impl<S, C, InputSpec, OutputSpec> rodio::Source for AudioStreamer<S, C, InputSpec, OutputSpec>
where
    S: Source + Iterator<Item = i16> + 'static,
    C: AnyComponent + 'static,
    InputSpec: EnumerateValues,
    OutputSpec: EnumerateValues + HasStorage<InternalWire>,
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
