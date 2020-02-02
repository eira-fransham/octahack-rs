use octahack::{
    octahack_components::{
        synth::{freq, output::Specifier as Out, params::Specifier as Params, Synth},
        OctahackComponent,
    },
    Rack, Value, WireDst, WireSrc,
};

octahack::specs! {
    mod any {
        OneChannel: octahack::Value
    }
}

impl Default for self::any::Params {
    fn default() -> Self {
        unimplemented!()
    }
}

use rodio::Source;

fn main() {
    let mut rack = Rack::<OctahackComponent, any::Specifier, any::Specifier>::new();

    let modulator_modulator = rack.new_component(Synth::new());
    let modulator = rack.new_component(Synth::new());
    let carrier = rack.new_component(Synth::new());
    rack.wire(
        WireSrc::component_output(modulator_modulator, Out::Saw),
        WireDst::component_param(modulator, Params::Freq, freq(55)),
    );
    rack.wire(
        WireSrc::component_output(modulator, Out::Sine),
        WireDst::component_param(carrier, Params::Freq, freq(880 * 2)),
    );
    rack.wire(
        WireSrc::component_output(carrier, Out::Square),
        WireDst::rack_output(any::Specifier::OneChannel),
    );
    rack.set_param::<_, Value>(modulator_modulator, Params::Freq, freq(0.5));
    rack.set_param::<_, Value>(modulator, Params::Freq, freq(220));
    rack.set_param::<_, Value>(carrier, Params::Freq, freq(440));

    println!("{}", rack);

    let streamer =
        octahack::output::AudioStreamer::new_convert(None, rack, rodio::source::SineWave::new(440))
            .convert_samples::<f32>();

    rodio::play_raw(&rodio::default_output_device().unwrap(), streamer);
    loop {}
}
