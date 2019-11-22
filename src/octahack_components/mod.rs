use crate::Value;

pub mod amplifier;
pub mod sine_synth;

use amplifier::Amplifier;
use sine_synth::SineSynth;

crate::component_set! {
    pub mod octahack_component {
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
        AnyComponent, AnyIter, QuickContext, Rack, RuntimeSpecifier, Value, ValueIter, WireDst,
        WireSrc,
    };

    crate::specs! {
        mod any {
            OneChannel: Value
        }
    }

    impl Default for self::any::Params {
        fn default() -> Self {
            unimplemented!()
        }
    }

    use self::any::Specifier;

    #[test]
    fn correct_max_out_size() {
        assert_eq!(
            super::amplifier::Specifier::TYPES.len(),
            super::OctahackComponent::MAX_OUTPUT_COUNT
        );
    }

    #[test]
    fn get_rack_output() {
        let mut rack = Rack::<super::OctahackComponent, Specifier, Specifier>::new();

        let amp = rack.new_component(super::Amplifier);
        rack.wire(
            WireSrc::rack_input(Specifier::OneChannel),
            WireDst::component_input(amp, super::amplifier::Specifier::Only),
        );
        rack.wire(
            WireSrc::component_output(amp, super::amplifier::Specifier::Only),
            WireDst::rack_output(Specifier::OneChannel),
        );

        let context = QuickContext::input(|_: &(), i| {
            Some(match i {
                Specifier::OneChannel => AnyIter::from(std::iter::once(Value::max_value())),
            })
        });

        rack.update(&context);
        assert_eq!(
            rack.output(Specifier::OneChannel, context)
                .map(|i| i.analog().unwrap().collect::<Vec<_>>()),
            Some(vec![Value::max_value()])
        );
    }

    #[test]
    fn circular_wiring() {
        use std::iter;

        let mut rack = Rack::<super::OctahackComponent, Specifier, Specifier>::new();

        let amp = rack.new_component(super::Amplifier);
        rack.wire(
            WireSrc::rack_input(Specifier::OneChannel),
            WireDst::component_input(amp, super::amplifier::Specifier::Only),
        );
        rack.wire(
            WireSrc::component_output(amp, super::amplifier::Specifier::Only),
            WireDst::component_input(amp, super::amplifier::Specifier::Only),
        );
        rack.set_param(amp, super::amplifier::Specifier::Only, Value::max_value());

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
