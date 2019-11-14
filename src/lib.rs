#![feature(type_alias_impl_trait, never_type)]

// TODO: Remove all uses of `Vec`, make `Value` generic

pub mod component;
pub mod octahack_components;
pub mod output;

pub use component::{
    AnyInputSpec, AnyIter, AnyOutputSpec, AnyParamSpec, Component, ComponentId, ComponentSet,
    ComponentSetOut, Context, GetInput, GetOutput, GetParam, Param, QuickContext, Rack, SpecId,
    Specifier, Types, Value, ValueIter, ValueKind, ValueType, Wire, WireDst, WireSrc,
};
