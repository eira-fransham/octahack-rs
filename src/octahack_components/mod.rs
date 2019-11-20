use crate::Value;

pub mod amplifier;
pub mod sine_synth;

use amplifier::Amplifier;
use sine_synth::SineSynth;

crate::component_set! {
    mod octahack_component {
        Amplifier,
        SineSynth
    }
}

pub use self::octahack_component::Component as OctahackComponent;

/// Equals `0.02`, which is a convenient number for us.
pub const VOLT: Value = Value::from_bits(0x28f5c29);

#[cfg(test)]
mod test {
    use crate::{
        octahack_components::amplifier::AmplifierIO, AnyComponent, AnyIter, QuickContext, Rack,
        SpecId, Specifier, Value, ValueIter, ValueType, WireDst, WireSrc,
    };
    

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    struct OneChannel;

    impl Specifier for OneChannel {
        const VALUES: &'static [Self] = &[OneChannel];
        const TYPES: &'static [ValueType] = &[ValueType::mono()];

        fn id(&self) -> SpecId {
            0
        }

        fn from_id(id: SpecId) -> Self {
            assert_eq!(id, 0);
            OneChannel
        }
    }

    #[test]
    fn correct_max_out_size() {
        assert_eq!(
            AmplifierIO::TYPES.len(),
            super::OctahackComponent::MAX_OUTPUT_COUNT
        );
    }

    #[test]
    fn get_rack_output() {
        let mut rack = Rack::<super::OctahackComponent, OneChannel, OneChannel>::new();

        let amp = rack.new_component(super::Amplifier);
        rack.wire(
            WireSrc::rack_input(OneChannel),
            WireDst::component_input(amp, AmplifierIO(0)),
        );
        rack.wire(
            WireSrc::component_output(amp, AmplifierIO(0)),
            WireDst::rack_output(OneChannel),
        );

        let context = QuickContext::input(|_: &(), i| {
            Some(match i {
                OneChannel => AnyIter::from(std::iter::once(Value::max_value())),
            })
        });

        rack.update(&context);
        assert_eq!(
            rack.output(OneChannel, context)
                .map(|i| i.analog().unwrap().collect::<Vec<_>>()),
            Some(vec![Value::max_value()])
        );
    }

    #[test]
    fn circular_wiring() {
        use std::iter;

        let mut rack = Rack::<super::OctahackComponent, OneChannel, OneChannel>::new();

        let amp = rack.new_component(super::Amplifier);
        rack.wire(
            WireSrc::rack_input(OneChannel),
            WireDst::component_input(amp, AmplifierIO(0)),
        );
        rack.wire(
            WireSrc::component_output(amp, AmplifierIO(0)),
            WireDst::component_input(amp, AmplifierIO(0)),
        );
        rack.set_param(amp, AmplifierIO(0), Value::max_value());

        let streamer = crate::output::AudioStreamer::new_convert(
            None,
            rack,
            rodio::source::SineWave::new(440),
        );

        assert_eq!(
            iter::repeat(0).take(100).collect::<Vec<_>>(),
            streamer.take(100).collect::<Vec<_>>()
        );
    }
}
