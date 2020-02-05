#![feature(trivial_bounds)]

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

    let modulator_modulator_func_id = rack.new_func();
    let mut func = rack.func_mut(modulator_modulator_func_id);

    let modulator_modulator = func.push_component(Synth::new());
    func.wire(
        WireSrc::component_output(modulator_modulator, Out::Saw),
        WireDst::rack_output(any::Specifier::OneChannel),
    );
    func.set_param::<_, Value>(modulator_modulator, Params::Freq, freq(0.5));

    let modulator_func_id = rack.new_func();
    let mut func = rack.func_mut(modulator_func_id);

    let modulator_modulator = func.push_function_call(modulator_modulator_func_id);
    let modulator = func.push_component(Synth::new());
    func.wire(
        WireSrc::component_output(modulator_modulator, any::Specifier::OneChannel),
        WireDst::component_param(modulator, Params::Freq, freq(55)),
    );
    func.wire(
        WireSrc::component_output(modulator, Out::Sine),
        WireDst::rack_output(any::Specifier::OneChannel),
    );
    func.set_param::<_, Value>(modulator, Params::Freq, freq(220));

    let carrier_func_id = rack.new_func();
    let mut func = rack.func_mut(carrier_func_id);

    let modulator = func.push_function_call(modulator_func_id);
    let carrier = func.push_component(Synth::new());
    func.wire(
        WireSrc::component_output(modulator, any::Specifier::OneChannel),
        WireDst::component_param(carrier, Params::Freq, freq(880 * 2)),
    );
    func.wire(
        WireSrc::component_output(carrier, Out::Square),
        WireDst::rack_output(any::Specifier::OneChannel),
    );
    func.set_param::<_, Value>(carrier, Params::Freq, freq(440));

    let mut main = rack.main_mut();
    let fcall = main.push_function_call(carrier_func_id);

    main.wire(
        WireSrc::component_output(fcall, any::Specifier::OneChannel),
        WireDst::rack_output(any::Specifier::OneChannel),
    );

    println!("{}", rack);

    let streamer =
        octahack::output::AudioStreamer::new_convert(None, rack, rodio::source::SineWave::new(440))
            .convert_samples::<f32>();

    rodio::play_raw(&rodio::default_output_device().unwrap(), streamer);
    loop {}
}
