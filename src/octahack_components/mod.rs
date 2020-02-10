use crate::Value;

pub mod amplifier;
pub mod file_player;
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

#[cfg(test)]
mod test {
    use crate::{Rack, Value, WireDst, WireSrc};

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
    fn circular_wiring() {
        use std::iter;

        let mut rack = Rack::<super::OctahackComponent, Specifier, Specifier>::new();

        let mut func = rack.main_mut();

        let amp = func.push_component(super::Amplifier);
        func.wire(
            WireSrc::func_input(Specifier::OneChannel),
            WireDst::component_input(amp, super::amplifier::input::Specifier::Input),
        );
        func.wire(
            WireSrc::component_output(amp, super::amplifier::output::Specifier::Output),
            WireDst::component_input(amp, super::amplifier::input::Specifier::Input),
        );
        func.set_param(amp, super::amplifier::params::Specifier::Amount, 1.);

        println!("{}", rack);

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
