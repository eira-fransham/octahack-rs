use octahack::{
    octahack_components::{
        sine_synth::{freq, SineSynth, SynthIO},
        OctahackComponent,
    },
    Rack, SpecId, Specifier, ValueType, WireDst, WireSrc,
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

use rodio::Source;

fn main() {
    let mut rack = Rack::<OctahackComponent, OneChannel, OneChannel>::new();

    let modulator_modulator = rack.new_component(SineSynth::new());
    let modulator = rack.new_component(SineSynth::new());
    let carrier = rack.new_component(SineSynth::new());
    rack.wire(
        WireSrc::component_output(modulator_modulator, SynthIO),
        WireDst::component_param(modulator, SynthIO, freq(55)),
    );
    rack.wire(
        WireSrc::component_output(modulator, SynthIO),
        WireDst::component_param(carrier, SynthIO, freq(880 * 2)),
    );
    rack.wire(
        WireSrc::component_output(carrier, SynthIO),
        WireDst::rack_output(OneChannel),
    );
    rack.set_param(modulator_modulator, SynthIO, freq(0.1));
    rack.set_param(modulator, SynthIO, freq(220));
    rack.set_param(carrier, SynthIO, freq(440));

    let streamer =
        octahack::output::AudioStreamer::new_convert(None, rack, rodio::source::SineWave::new(440))
            .convert_samples::<f32>();

    rodio::play_raw(&rodio::default_output_device().unwrap(), streamer);
    loop {}
}
