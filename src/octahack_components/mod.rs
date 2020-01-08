use crate::Value;

pub mod amplifier;
// pub mod file_player;
pub mod midi_expander;
pub mod synth;

use amplifier::Amplifier;
use synth::Synth;

crate::component_set! {
    pub mod octahack_component {
        Amplifier,
        Synth
    }
}

pub use self::octahack_component::Component as OctahackComponent;

/// Equals `0.02` for `fixed::I1F31`
pub const VOLT: Value = Value::from_bits(0x02_8f_5c_29);

#[cfg(test)]
mod test {
    use crate::{AnyComponent, Rack, RuntimeSpecifier, Value, WireDst, WireSrc};

    crate::specs! {
        mod any {
            OneChannel: crate::Value
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
            super::synth::output::Specifier::TYPES.len(),
            super::OctahackComponent::MAX_OUTPUT_COUNT
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

        let mut streamer = crate::output::AudioStreamer::new_convert(
            None,
            rack,
            rodio::source::SineWave::new(440),
        );

        Iterator::next(&mut streamer);

        assert_eq!(
            iter::repeat(0).take(100).collect::<Vec<_>>(),
            Iterator::take(streamer, 100).collect::<Vec<_>>()
        );
    }
}
