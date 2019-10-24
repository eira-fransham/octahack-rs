use nom_midi::MidiEventType;
use std::{cell::Cell, marker::PhantomData};
use typenum::consts;

pub type Continuous = fixed::FixedI32<consts::U32>;
pub type Continuous16 = fixed::FixedI16<consts::U16>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ValueType {
    Binary,
    Continuous,
    // The inner U8 is the maximum
    Discrete(u8),
    Midi,
}

// Used for both parameters and I/O, since we want to allow wiring outputs to params
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Value {
    Binary(bool),
    Continuous(Continuous),
    Discrete(u8),
    Midi(MidiEventType),
}

impl Value {
    pub fn continuous(self) -> Option<Continuous> {
        match self {
            Value::Continuous(val) => Some(val),
            _ => None,
        }
    }

    pub fn discrete(self) -> Option<u8> {
        match self {
            Value::Discrete(val) => Some(val),
            _ => None,
        }
    }

    pub fn midi(self) -> Option<MidiEventType> {
        match self {
            Value::Midi(val) => Some(val),
            _ => None,
        }
    }
}

impl From<Continuous> for Value {
    fn from(other: Continuous) -> Self {
        Value::Continuous(other)
    }
}

impl From<f32> for Value {
    fn from(other: f32) -> Self {
        Value::Continuous(Continuous::saturating_from_num(other))
    }
}

// TODO: This can probably be `u8`
pub type SpecId = usize;

pub trait Specifier: Sized + Clone + 'static {
    const VALUES: &'static [Self];
    // TODO: This should just be a stopgap until `const fn`s are more fleshed-out - at the
    //       moment it's not possible to define this as `VALUES.map(Self::typeof)`.
    const TYPES: &'static [ValueType];

    fn value_type(&self) -> ValueType {
        Self::TYPES[self.id()]
    }

    fn id(&self) -> SpecId;
    fn from_id(id: SpecId) -> Self {
        Self::VALUES[id].clone()
    }
}

pub trait Param: Specifier {
    fn default(&self) -> Value;
}

pub trait Component {
    type InputSpecifier: Specifier;
    type OutputSpecifier: Specifier;
    type ParamSpecifier: Specifier;

    fn output<Ctx>(&self, id: Self::OutputSpecifier, ctx: &mut Ctx) -> Option<Value>
    where
        for<'a> &'a mut Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>;

    fn update<Ctx>(&mut self, _ctx: &mut Ctx)
    where
        for<'a> &'a mut Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>,
    {
    }
}

pub trait GetInput<Spec> {
    fn input(self, spec: Spec) -> Option<Value>;
}

pub trait GetParam<Spec> {
    fn param(self, spec: Spec) -> Value;
}

#[macro_export]
macro_rules! component_set {
    ($v:vis mod $name:ident { $($t:ident),* }) => {
        #[allow(dead_code)]
        $v mod $name {
            pub enum Component {
                $($t(super::$t)),*
            }

            pub enum Input {
                $($t(<super::$t as $crate::Component>::InputSpecifier)),*
            }
            pub enum Output {
                $($t(<super::$t as $crate::Component>::OutputSpecifier)),*
            }
            pub enum Param {
                $($t(<super::$t as $crate::Component>::ParamSpecifier)),*
            }

            $(
                impl From<super::$t> for Component {
                    fn from(other: super::$t) -> Self {
                        Component::$t(other)
                    }
                }

                impl From<<super::$t as $crate::Component>::InputSpecifier> for Input {
                    fn from(other: <super::$t as $crate::Component>::InputSpecifier) -> Self {
                        Self::$t(other)
                    }
                }

                impl From<<super::$t as $crate::Component>::OutputSpecifier> for Output {
                    fn from(other: <super::$t as $crate::Component>::OutputSpecifier) -> Self {
                        Self::$t(other)
                    }
                }

                impl From<<super::$t as $crate::Component>::ParamSpecifier> for Param {
                    fn from(other: <super::$t as $crate::Component>::ParamSpecifier) -> Self {
                        Self::$t(other)
                    }
                }
            )*

            enum ParamDefaultsInner {
                $($t(&'static [<super::$t as $crate::Component>::ParamSpecifier])),*
            }

            #[doc(hidden)]
            pub struct ParamDefaults {
                inner: ParamDefaultsInner,
            }

            impl Iterator for ParamDefaults
            where
            $(
                <super::$t as $crate::Component>::ParamSpecifier: $crate::Param
            )*
            {
                type Item = $crate::Value;

                fn next(&mut self) -> Option<Self::Item> {
                    match &mut self.inner {
                        $(
                            ParamDefaultsInner::$t(inner) => {
                                let (first, rest) = inner.split_first()?;

                                *inner = rest;

                                Some($crate::Param::default(first))
                            },
                        )*
                    }

                }
            }

            impl $crate::ComponentSet for Component {
                const MAX_OUTPUT_COUNT: usize = {
                    let mut out = 0;

                    $(
                        {
                            let count = {
                                <
                                    <super::$t as $crate::Component>::OutputSpecifier as
                                        $crate::Specifier
                                >::VALUES.len()
                            };
                            // `0xFFFFFFFF` if count > out, 0 otherwise
                            let out_mask = (!(count > out) as usize).wrapping_sub(1);

                            out = (!out_mask & out) | (out_mask & count);
                        }
                    )*

                    out
                };

                type ParamDefaults = ParamDefaults;

                fn types(&self) -> $crate::Types {
                    match self {
                        $(
                            Self::$t(_) => {
                                $crate::Types {
                                input: <<super::$t as $crate::Component>::InputSpecifier as $crate::Specifier>::TYPES,
                                output: <<super::$t as $crate::Component>::OutputSpecifier as $crate::Specifier>::TYPES,
                                parameters:<<super::$t as $crate::Component>::ParamSpecifier as $crate::Specifier>::TYPES,
                                }
                            },
                        )*
                    }
                }

                fn param_defaults(&self) -> Self::ParamDefaults {
                    match self {
                        $(
                            Self::$t(_) => {
                                ParamDefaults {
                                    inner: ParamDefaultsInner::$t(
                                        <
                                            <super::$t as $crate::Component>::OutputSpecifier as
                                                $crate::Specifier
                                        >::VALUES
                                    )
                                }
                            },
                        )*
                    }
                }

                fn output<Ctx>(&self, id: $crate::AnyOutputSpec, ctx: &mut Ctx) -> Option<$crate::Value>
                where
                    for<'a> &'a mut Ctx: $crate::GetInput<$crate::AnyInputSpec> + $crate::GetParam<$crate::AnyParamSpec>
                {
                    match self {
                        $(
                            Self::$t(val) => {
                                use $crate::{Component, GetInput, GetParam, Specifier};

                                val.output(
                                    <<super::$t as $crate::Component>::OutputSpecifier as $crate::Specifier>::from_id(id.0),
                                    &mut $crate::component::QuickContext::new(
                                        ctx,
                                        |ctx: &mut Ctx, spec: <super::$t as $crate::Component>::InputSpecifier| {
                                            ctx.input($crate::AnyInputSpec(spec.id()))
                                        },
                                        |ctx: &mut Ctx, spec: <super::$t as $crate::Component>::ParamSpecifier| ctx.param($crate::AnyParamSpec(spec.id())),
                                    )
                                )
                            },
                        )*
                    }
                }

                fn update<Ctx>(&mut self, ctx: &mut Ctx)
                where
                    for<'a> &'a mut Ctx: $crate::GetInput<$crate::AnyInputSpec> + $crate::GetParam<$crate::AnyParamSpec>
                {
                    match self {
                        $(
                            Self::$t(val) => {
                                use $crate::{Component, GetInput, GetParam, Specifier};

                                val.update(
                                    &mut $crate::component::QuickContext::new(
                                        ctx,
                                        |ctx: &mut Ctx, spec: <super::$t as $crate::Component>::InputSpecifier| {
                                            ctx.input($crate::AnyInputSpec(spec.id()))
                                        },
                                        |ctx: &mut Ctx, spec: <super::$t as $crate::Component>::ParamSpecifier| ctx.param($crate::AnyParamSpec(spec.id())),
                                    )
                                )
                            },
                        )*
                    }
                }
            }
        }
    }
}

pub struct AnyOutputSpec(pub SpecId);
pub struct AnyInputSpec(pub SpecId);
pub struct AnyParamSpec(pub SpecId);

pub struct Types {
    pub input: &'static [ValueType],
    pub output: &'static [ValueType],
    pub parameters: &'static [ValueType],
}

pub trait ComponentSet {
    const MAX_OUTPUT_COUNT: usize;

    type ParamDefaults: Iterator<Item = Value>;

    fn types(&self) -> Types;

    fn param_defaults(&self) -> Self::ParamDefaults;

    fn output<Ctx>(&self, id: AnyOutputSpec, ctx: &mut Ctx) -> Option<Value>
    where
        for<'a> &'a mut Ctx: GetInput<AnyInputSpec> + GetParam<AnyParamSpec>;

    fn update<Ctx>(&mut self, ctx: &mut Ctx)
    where
        for<'a> &'a mut Ctx: GetInput<AnyInputSpec> + GetParam<AnyParamSpec>;
}

pub struct QuickContext<C, InputFn, ParamFn> {
    ctx: C,
    input_fn: InputFn,
    param_fn: ParamFn,
}

impl<F, Spec> GetInput<Spec> for &'_ mut F
where
    F: FnMut(Spec) -> Option<Value>,
{
    fn input(self, spec: Spec) -> Option<Value> {
        self(spec)
    }
}

impl<F, Spec> GetInput<Spec> for &'_ F
where
    F: Fn(Spec) -> Option<Value>,
{
    fn input(self, spec: Spec) -> Option<Value> {
        self(spec)
    }
}

impl<C, InputFn, ParamFn> QuickContext<C, InputFn, ParamFn> {
    pub fn new(ctx: C, input_fn: InputFn, param_fn: ParamFn) -> Self {
        QuickContext {
            ctx,
            input_fn,
            param_fn,
        }
    }
}

impl<C, InputFn, ParamFn, Spec> GetInput<Spec> for &'_ mut QuickContext<&'_ mut C, InputFn, ParamFn>
where
    InputFn: FnMut(&mut C, Spec) -> Option<Value>,
{
    fn input(self, spec: Spec) -> Option<Value> {
        (self.input_fn)(self.ctx, spec)
    }
}

impl<C, InputFn, ParamFn, Spec> GetParam<Spec> for &'_ mut QuickContext<&'_ mut C, InputFn, ParamFn>
where
    ParamFn: FnMut(&mut C, Spec) -> Value,
{
    fn param(self, spec: Spec) -> Value {
        (self.param_fn)(self.ctx, spec)
    }
}

impl<C, InputFn, ParamFn, Spec> GetInput<Spec> for &'_ mut QuickContext<&'_ C, InputFn, ParamFn>
where
    InputFn: FnMut(&C, Spec) -> Option<Value>,
{
    fn input(self, spec: Spec) -> Option<Value> {
        (self.input_fn)(self.ctx, spec)
    }
}

impl<C, InputFn, ParamFn, Spec> GetParam<Spec> for &'_ mut QuickContext<&'_ C, InputFn, ParamFn>
where
    ParamFn: FnMut(&C, Spec) -> Value,
{
    fn param(self, spec: Spec) -> Value {
        (self.param_fn)(self.ctx, spec)
    }
}

impl<C, InputFn, ParamFn, Spec> GetInput<Spec> for &'_ QuickContext<C, InputFn, ParamFn>
where
    C: Copy,
    InputFn: Fn(C, Spec) -> Option<Value>,
{
    fn input(self, spec: Spec) -> Option<Value> {
        (self.input_fn)(self.ctx, spec)
    }
}

impl<C, InputFn, ParamFn, Spec> GetParam<Spec> for &'_ QuickContext<C, InputFn, ParamFn>
where
    C: Copy,
    ParamFn: Fn(C, Spec) -> Value,
{
    fn param(self, spec: Spec) -> Value {
        (self.param_fn)(self.ctx, spec)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ElementSpecifier<Id> {
    Component { id: Id, index: usize },
    Rack,
}

impl<Id> ElementSpecifier<Id> {
    fn fill_id<NewId>(self, f: impl FnOnce(usize) -> NewId) -> ElementSpecifier<NewId> {
        match self {
            Self::Component { id: _, index } => ElementSpecifier::Component {
                id: f(index),
                index,
            },
            Self::Rack => ElementSpecifier::Rack,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Input;
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Output;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct GenericWire<Marker, Id> {
    io_index: SpecId,
    element: ElementSpecifier<Id>,
    _marker: PhantomData<Marker>,
}

impl<M> NewWire<M> {
    fn fill_id(self, f: impl FnOnce(usize) -> ComponentId) -> Wire<M> {
        Wire(GenericWire {
            io_index: self.0.io_index,
            element: self.0.element.fill_id(f),
            _marker: PhantomData,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Wire<Marker>(GenericWire<Marker, ComponentId>);
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NewWire<Marker>(GenericWire<Marker, ()>);

impl NewWire<Input> {
    pub fn rack_output<S: Specifier>(output: S) -> Self {
        NewWire(GenericWire {
            io_index: output.id(),
            element: ElementSpecifier::Rack,
            _marker: PhantomData,
        })
    }

    pub fn component_input<S: Specifier>(component_index: usize, input: S) -> Self {
        NewWire(GenericWire {
            io_index: input.id(),
            element: ElementSpecifier::Component {
                id: (),
                index: component_index,
            },
            _marker: PhantomData,
        })
    }
}

impl NewWire<Output> {
    pub fn rack_input<S: Specifier>(input: S) -> Self {
        NewWire(GenericWire {
            io_index: input.id(),
            element: ElementSpecifier::Rack,
            _marker: PhantomData,
        })
    }

    pub fn component_output<S: Specifier>(component_index: usize, output: S) -> Self {
        NewWire(GenericWire {
            io_index: output.id(),
            element: ElementSpecifier::Component {
                id: (),
                index: component_index,
            },
            _marker: PhantomData,
        })
    }
}

impl<M, Id> GenericWire<M, Id>
where
    ElementSpecifier<Id>: Copy,
{
    fn element(&self) -> ElementSpecifier<Id> {
        self.element
    }
}

// TODO: Also allow this to be a parameter somehow
impl<Id> GenericWire<Input, Id> {
    fn input_id(&self) -> AnyInputSpec {
        AnyInputSpec(self.io_index)
    }
}

impl<Id> GenericWire<Output, Id> {
    fn output_id(&self) -> AnyOutputSpec {
        AnyOutputSpec(self.io_index)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ComponentId(usize);

#[derive(Debug, Clone, PartialEq)]
struct TaggedComponent<C> {
    id: ComponentId,
    inner: C,
    params: Vec<Value>,
    wires: Vec<Option<Wire<Output>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RackComponent<C> {
    Component(C),
    Group {
        inputs: Vec<ValueType>,
        outputs: Vec<ValueType>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct ComponentIdGen {
    cur: usize,
}

impl ComponentIdGen {
    fn new() -> Self {
        Self { cur: 0 }
    }

    fn next(&mut self) -> ComponentId {
        let cur = self.cur;
        self.cur += 1;
        ComponentId(cur)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rack<C, InputSpec, OutputSpec> {
    ids: ComponentIdGen,
    previous_outputs: Vec<Vec<Cell<SavedValue>>>,
    current_outputs: Vec<Vec<Cell<SavedValue>>>,
    components: Vec<TaggedComponent<C>>,
    out_wires: Vec<Option<Wire<Output>>>,
    _marker: PhantomData<(InputSpec, OutputSpec)>,
}

impl<C, InputSpec, OutputSpec> Rack<C, InputSpec, OutputSpec>
where
    InputSpec: Specifier,
    OutputSpec: Specifier,
    C: ComponentSet,
{
    pub fn new() -> Self {
        use std::iter;

        Rack {
            ids: ComponentIdGen::new(),
            previous_outputs: vec![],
            current_outputs: vec![],
            components: vec![],
            out_wires: iter::repeat(None).take(OutputSpec::TYPES.len()).collect(),
            _marker: PhantomData,
        }
    }

    pub fn update<Ctx>(&mut self, ctx: &Ctx)
    where
        for<'a> &'a Ctx: GetInput<InputSpec>,
    {
        self.as_slice().update(ctx);
        std::mem::swap(&mut self.previous_outputs, &mut self.current_outputs);
    }

    // TODO: Return a result
    pub fn wire(&mut self, output: NewWire<Output>, input: NewWire<Input>) {
        let filled_output = output.fill_id(|index| self.components[index].id);
        match input.0.element() {
            ElementSpecifier::Component { id: _, index } => {
                self.components[index].wires[input.0.input_id().0] = Some(filled_output);
            }
            ElementSpecifier::Rack => self.out_wires[input.0.input_id().0] = Some(filled_output),
        };
    }

    pub fn set_param<S: Specifier, V: Into<Value>>(
        &mut self,
        component: usize,
        param: S,
        value: V,
    ) {
        self.components[component].params[param.id()] = value.into();
    }

    pub fn new_component(&mut self, component: impl Into<C>) -> usize {
        let component = component.into();
        let out = self.components.len();
        let params = component.param_defaults();
        let num_inputs = component.types().input.len();

        self.components.push(TaggedComponent {
            id: self.ids.next(),
            inner: component,
            wires: vec![None; num_inputs],
            params: params.collect(),
        });

        out
    }

    /// Get a specific output of this rack. This takes a mutable reference because it technically
    /// isn't pure, but it _is_ idempotent.
    pub fn output<Ctx>(&mut self, spec: OutputSpec, ctx: &Ctx) -> Option<Value>
    where
        for<'a> &'a Ctx: GetInput<InputSpec>,
    {
        use std::iter;

        let wire = self.out_wires[spec.id()]?;

        for o in &mut self.current_outputs {
            o.clear();
        }

        if self.current_outputs.len() < self.components.len() {
            self.current_outputs.extend(
                iter::repeat(vec![]).take(self.components.len() - self.current_outputs.len()),
            );
        }

        let out = self.as_slice().output(wire, ctx);
        out
    }

    fn as_slice(&mut self) -> RackSlice<'_, C, InputSpec> {
        RackSlice {
            previous_outputs: &self.previous_outputs,
            current_outputs: &mut self.current_outputs,
            current_component: None,
            components: &mut self.components,
            _marker: PhantomData::<InputSpec>,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum SavedValue {
    Calculating,
    NotCalculated,
    Calculated(Option<Value>),
}

impl SavedValue {
    fn to_option(self) -> Option<Value> {
        match self {
            Self::NotCalculated | Self::Calculating => None,
            Self::Calculated(val) => val,
        }
    }
}

struct RackSlice<'a, C, InputSpec> {
    previous_outputs: &'a [Vec<Cell<SavedValue>>],
    current_outputs: &'a mut [Vec<Cell<SavedValue>>],
    current_component: Option<(&'a TaggedComponent<C>, &'a [Cell<SavedValue>])>,
    components: &'a mut [TaggedComponent<C>],
    _marker: PhantomData<InputSpec>,
}

impl<C, InputSpec> RackSlice<'_, C, InputSpec>
where
    C: ComponentSet,
    InputSpec: Specifier,
{
    fn update<Ctx>(&mut self, ctx: &Ctx)
    where
        for<'a> &'a Ctx: GetInput<InputSpec>,
    {
        for index in (0..self.components.len()).rev() {
            let (rest, this) = self.components.split_at_mut(index);
            let this = &mut this[0];

            let inner = &mut this.inner;
            let wires = &this.wires;
            let params = &this.params;

            let previous_outputs = self.previous_outputs;
            let current_outputs = &mut *self.current_outputs;

            inner.update(&mut QuickContext::new(
                ctx,
                |ctx: &Ctx, spec: AnyInputSpec| {
                    let wire = wires[spec.0]?;

                    RackSlice {
                        previous_outputs,
                        current_outputs,
                        current_component: None,
                        components: rest,
                        _marker: PhantomData,
                    }
                    .output(wire, ctx)
                },
                |_: &Ctx, spec: AnyParamSpec| params[spec.0],
            ));
        }
    }

    fn output<Ctx>(&mut self, Wire(wire): Wire<Output>, ctx: &Ctx) -> Option<Value>
    where
        for<'a> &'a Ctx: GetInput<InputSpec>,
    {
        use std::iter;

        match wire.element() {
            ElementSpecifier::Component { id, index } => {
                match self
                    .current_outputs
                    .get(index)
                    .and_then(|out| out.get(wire.output_id().0))
                    .map(|cell| cell.get())
                {
                    Some(SavedValue::Calculating) => return None,
                    Some(SavedValue::Calculated(val)) => return val,
                    None | Some(SavedValue::NotCalculated) => {}
                }

                let saved_out = self
                    .previous_outputs
                    .get(index)
                    .and_then(|outs| outs.get(wire.output_id().0))
                    .and_then(|out| out.get().to_option());

                // This complicated if-else chain is to support components being wired to themselves
                let (rest, this, rest_outputs, this_output) = if index < self.components.len() {
                    let (rest, this) = self.components.split_at_mut(index);
                    let (rest_outputs, this_output) = self.current_outputs.split_at_mut(index);

                    let this_output = &mut this_output[0];

                    if this_output.len() <= wire.output_id().0 {
                        this_output.extend(
                            iter::repeat(Cell::new(SavedValue::NotCalculated))
                                .take(wire.output_id().0 + 1 - this_output.len()),
                        );
                    }

                    let this_output: &[_] = &*this_output;

                    (rest, &this[0], rest_outputs, this_output)
                } else if index == self.components.len() {
                    if let Some((comp, out)) = self.current_component {
                        (&mut *self.components, comp, &mut *self.current_outputs, out)
                    } else {
                        return saved_out;
                    }
                } else {
                    return saved_out;
                };

                if this.id != id {
                    // TODO: Disconnect this wire too?
                    return None;
                }

                let previous_outputs = self.previous_outputs;

                let inner = &this.inner;
                let wires = &this.wires;
                let params = &this.params;

                this_output[wire.output_id().0].set(SavedValue::Calculating);

                let out = inner.output(
                    AnyOutputSpec(wire.output_id().0),
                    &mut QuickContext::new(
                        ctx,
                        |ctx: &Ctx, spec: AnyInputSpec| {
                            let wire = wires[spec.0]?;

                            RackSlice {
                                previous_outputs,
                                current_outputs: rest_outputs,
                                current_component: Some((this, this_output)),
                                components: rest,
                                _marker: PhantomData,
                            }
                            .output(wire, ctx)
                        },
                        |_: &Ctx, spec: AnyParamSpec| params[spec.0],
                    ),
                );

                this_output[wire.output_id().0].set(SavedValue::Calculated(out));
                out
            }
            ElementSpecifier::Rack => ctx.input(InputSpec::from_id(wire.io_index)),
        }
    }
}
