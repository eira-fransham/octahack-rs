#![feature(trivial_bounds)]

use octahack::{
    octahack_components::{
        synth::{freq, output::Specifier as Out, params::Specifier as Params, Synth},
        OctahackComponent,
    },
    rack::{AsParam, Param, Settable},
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

    let mut main = rack.main_mut();

    let cv_modulator = main.push_component(Synth::new());
    main.set_param(cv_modulator, Params::Freq, freq(0.2));
    let modulator = main.push_component(Synth::new());
    let carrier = main.push_component(Synth::new());
    main.wire(
        WireSrc::component_output(carrier, Out::Sine),
        WireDst::func_output(any::Specifier::OneChannel),
    );
    main.set_param(modulator, Params::Freq, freq(440));

    {
        let mut carrier_freq = main
            .param::<_, Value>(carrier, Params::Freq)
            .as_param()
            .unwrap();
        carrier_freq.set(freq(220.));
        carrier_freq.wire(WireSrc::component_output(modulator, Out::Sine), 1.);
        carrier_freq
            .cv()
            .unwrap()
            .wire(WireSrc::component_output(cv_modulator, Out::Sine), 4.);
    }

    // println!("{}", rack);

    let streamer =
        octahack::output::AudioStreamer::new_convert(None, rack, rodio::source::SineWave::new(440))
            .convert_samples::<f32>();

    rodio::play_raw(&rodio::default_output_device().unwrap(), streamer);
    loop {}
}
