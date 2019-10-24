// TODO: Remove all uses of `Vec`, make `Value` generic

mod component;
mod octahack_components;
mod output;

pub use component::{
    AnyInputSpec, AnyOutputSpec, AnyParamSpec, Component, ComponentId, ComponentSet, Continuous,
    Continuous16, GetInput, GetParam, NewWire, Param, Rack, SpecId, Specifier, Types, Value,
    ValueType, Wire,
};
