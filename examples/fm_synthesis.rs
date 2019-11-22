use octahack::{
    octahack_components::{
        sine_synth::{freq, SineSynth, Specifier::IO as SynthIO},
        OctahackComponent,
    },
    Rack, Value, WireDst, WireSrc,
};

octahack::specs! {
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

use rodio::Source;

fn main() {
    let mut rack = Rack::<OctahackComponent, Specifier, Specifier>::new();

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
        WireDst::rack_output(Specifier::OneChannel),
    );
    rack.set_param::<_, Value>(modulator_modulator, SynthIO, freq(0.1));
    rack.set_param::<_, Value>(modulator, SynthIO, freq(220));
    rack.set_param::<_, Value>(carrier, SynthIO, freq(440));

    let streamer =
        octahack::output::AudioStreamer::new_convert(None, rack, rodio::source::SineWave::new(440))
            .convert_samples::<f32>();

    rodio::play_raw(&rodio::default_output_device().unwrap(), streamer);
    loop {}
}
