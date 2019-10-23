use crate::{Component, GetInput, GetParam, SpecId, Specifier, Value, ValueType};
use cpal::Sample as CpalSample;
use rodio::{Sample, Source};

pub struct Amplifier;

#[derive(Copy, Clone)]
pub struct AmplifierIO(u8);

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

impl Component for Amplifier {
    type InputSpecifier = AmplifierIO;
    type OutputSpecifier = AmplifierIO;
    type ParamSpecifier = AmplifierIO;

    fn output<Ctx>(&mut self, id: Self::OutputSpecifier, ctx: &mut Ctx) -> Option<Value>
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

        Some(Value::from(to_multiply * multiplication_factor))
    }
}

crate::component_set! {
    mod octahack_component {
        Amplifier
    }
}

pub use self::octahack_component::Component as OctahackComponent;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum OctahackInput {
    AudioA,
    AudioB,
    AudioC,
    AudioD,
}

impl Specifier for OctahackInput {
    const VALUES: &'static [Self] = &[
        OctahackInput::AudioA,
        OctahackInput::AudioB,
        OctahackInput::AudioC,
        OctahackInput::AudioD,
    ];
    const TYPES: &'static [ValueType] = &[ValueType::Continuous; 4];

    fn id(&self) -> SpecId {
        *self as _
    }

    fn from_id(id: SpecId) -> Self {
        match id {
            0 => Self::AudioA,
            1 => Self::AudioB,
            2 => Self::AudioC,
            3 => Self::AudioD,
            _ => panic!("Invalid id for `RackOut`"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum OctahackOutput {
    MainL,
    MainR,
    CueL,
    CueR,
}

impl Specifier for OctahackOutput {
    const VALUES: &'static [Self] = &[
        OctahackOutput::MainL,
        OctahackOutput::MainR,
        OctahackOutput::CueL,
        OctahackOutput::CueR,
    ];
    const TYPES: &'static [ValueType] = &[ValueType::Continuous; 4];

    fn id(&self) -> SpecId {
        *self as _
    }
}

struct Octahack<AudioA, AudioB, AudioC, AudioD>
where
    AudioA: rodio::Source,
    AudioB: rodio::Source,
    AudioC: rodio::Source,
    AudioD: rodio::Source,

    AudioA::Item: rodio::Sample,
    AudioB::Item: rodio::Sample,
    AudioC::Item: rodio::Sample,
    AudioD::Item: rodio::Sample,
{
    audio_inputs: (AudioA, AudioB, AudioC, AudioD),
    last_tick: (
        Option<AudioA::Item>,
        Option<AudioB::Item>,
        Option<AudioC::Item>,
        Option<AudioD::Item>,
    ),
    rack: crate::Rack<OctahackComponent, OctahackInput, OctahackOutput>,
}

impl<AudioA, AudioB, AudioC, AudioD> Octahack<AudioA, AudioB, AudioC, AudioD>
where
    AudioA: Source,
    AudioB: Source,
    AudioC: Source,
    AudioD: Source,

    AudioA::Item: Sample,
    AudioB::Item: Sample,
    AudioC::Item: Sample,
    AudioD::Item: Sample,
{
    fn ctx(
        (audio_a, audio_b, audio_c, audio_d): &(
            Option<AudioA::Item>,
            Option<AudioB::Item>,
            Option<AudioC::Item>,
            Option<AudioD::Item>,
        ),
    ) -> impl Fn(OctahackInput) -> Option<Value> + '_ {
        move |i| match i {
            OctahackInput::AudioA => audio_a
                .map(|val| Value::Continuous(crate::Continuous::from_bits(val.to_i16() as _))),
            OctahackInput::AudioB => audio_b
                .map(|val| Value::Continuous(crate::Continuous::from_bits(val.to_i16() as _))),
            OctahackInput::AudioC => audio_c
                .map(|val| Value::Continuous(crate::Continuous::from_bits(val.to_i16() as _))),
            OctahackInput::AudioD => audio_d
                .map(|val| Value::Continuous(crate::Continuous::from_bits(val.to_i16() as _))),
        }
    }

    fn update(&mut self) {
        self.last_tick = (
            self.audio_inputs.0.next(),
            self.audio_inputs.1.next(),
            self.audio_inputs.2.next(),
            self.audio_inputs.3.next(),
        );

        self.rack.update(&Self::ctx(&self.last_tick));
    }
}

#[cfg(test)]
mod test {
    use crate::{ComponentSet, NewWire, Rack, SpecId, Specifier, Value, ValueType};

    #[test]
    fn correct_max_out_size() {
        assert_eq!(
            super::AmplifierIO::TYPES.len(),
            super::OctahackComponent::MAX_OUTPUT_COUNT
        );
    }

    #[test]
    fn get_rack_output() {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        enum RackOut {
            Audio,
            Midi,
        }

        impl Specifier for RackOut {
            const VALUES: &'static [Self] = &[RackOut::Audio, RackOut::Midi];
            const TYPES: &'static [ValueType] = &[ValueType::Continuous, ValueType::Midi];

            fn id(&self) -> SpecId {
                match self {
                    Self::Audio => 0,
                    Self::Midi => 1,
                }
            }

            fn from_id(id: SpecId) -> Self {
                match id {
                    0 => Self::Audio,
                    1 => Self::Midi,
                    _ => panic!("Invalid id for `RackOut`"),
                }
            }
        }

        let mut rack = Rack::<super::OctahackComponent, RackOut, RackOut>::new();

        let amp = rack.new_component(super::Amplifier);
        rack.wire(
            NewWire::rack_input(RackOut::Audio),
            NewWire::component_input(amp, super::AmplifierIO(0)),
        );
        rack.wire(
            NewWire::component_output(amp, super::AmplifierIO(0)),
            NewWire::rack_output(RackOut::Audio),
        );

        let context = |i| {
            Some(match i {
                RackOut::Audio => Value::Continuous(crate::Continuous::max_value()),
                RackOut::Midi => Value::Midi,
            })
        };

        rack.update(&context);
        assert_eq!(
            rack.output(RackOut::Audio, &context),
            Some(Value::Continuous(crate::Continuous::default()))
        );
    }

    #[test]
    fn stream_audio() {
        use rodio::Source;

        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        struct OneChannel;

        impl Specifier for OneChannel {
            const VALUES: &'static [Self] = &[OneChannel];
            const TYPES: &'static [ValueType] = &[ValueType::Continuous];

            fn id(&self) -> SpecId {
                0
            }

            fn from_id(id: SpecId) -> Self {
                assert_eq!(id, 0);
                OneChannel
            }
        }

        let mut rack = Rack::<super::OctahackComponent, OneChannel, OneChannel>::new();

        let amp = rack.new_component(super::Amplifier);
        rack.wire(
            NewWire::rack_input(OneChannel),
            NewWire::component_input(amp, super::AmplifierIO(0)),
        );
        rack.wire(
            NewWire::component_output(amp, super::AmplifierIO(0)),
            NewWire::rack_output(OneChannel),
        );
        rack.set_param(amp, super::AmplifierIO(0), 0.01);

        let mut streamer = crate::output::AudioStreamer::new_convert(
            None,
            rack,
            rodio::source::SineWave::new(440),
        );

        rodio::play_raw(
            &rodio::default_output_device().unwrap(),
            streamer.convert_samples(),
        );

        loop {}
    }
}
