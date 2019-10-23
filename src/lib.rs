#![feature(const_slice_len)]

// TODO: Remove all uses of `Vec`, make `Value` generic

mod component;
mod octahack_components;
mod output;

pub use component::{
    AnyInputSpec, AnyOutputSpec, AnyParamSpec, Component, ComponentId, ComponentSet, Continuous,
    Continuous16, GetInput, GetParam, NewWire, Rack, SpecId, Specifier, Types, Value, ValueType,
    Wire,
};
