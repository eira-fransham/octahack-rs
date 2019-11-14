use fixed::types::{I0F32, U0F32};
use itertools::Either;
use nom_midi::MidiEventType;
use std::{iter::ExactSizeIterator, marker::PhantomData};
use typenum::consts;

fn u_to_s(unsigned: U0F32) -> I0F32 {
    I0F32::from_bits(
        unsigned
            .to_bits()
            .wrapping_sub(I0F32::max_value().to_bits() as u32) as _,
    )
}

fn s_to_u(signed: I0F32) -> fixed::FixedU32<consts::U32> {
    fixed::FixedU32::<consts::U32>::from_bits(
        (signed.to_bits() as u32).wrapping_add(I0F32::max_value().to_bits() as u32) as _,
    )
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ValueKind {
    Binary,
    Continuous,
    // The inner U8 is the maximum
    Discrete(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ValueType {
    pub kind: ValueKind,
    pub channels: Option<u8>,
}

impl ValueType {
    pub const fn continuous() -> Self {
        ValueType {
            kind: ValueKind::Continuous,
            channels: None,
        }
    }

    pub const fn mono() -> Self {
        ValueType {
            kind: ValueKind::Continuous,
            channels: Some(1),
        }
    }

    pub const fn stereo() -> Self {
        ValueType {
            kind: ValueKind::Continuous,
            channels: Some(2),
        }
    }
}

pub type Value = I0F32;

pub trait ValueExt {
    fn discrete(self, max: u8) -> u8;
}

impl ValueExt for I0F32 {
    fn discrete(self, max: u8) -> u8 {
        (f64::from(self) * max as f64) as u8
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

impl Specifier for ! {
    const VALUES: &'static [Self] = &[];
    const TYPES: &'static [ValueType] = &[];

    fn id(&self) -> SpecId {
        unreachable!()
    }
}

pub trait Param: Specifier {
    fn default(&self) -> Value;
}

pub trait Component {
    type InputSpecifier: Specifier;
    type OutputSpecifier: Specifier;
    type ParamSpecifier: Specifier;
}

// TODO: Support MIDI inputs

pub trait ValueIter {
    type Midi: ExactSizeIterator<Item = MidiEventType> + Send;
    type Analog: ExactSizeIterator<Item = I0F32> + Send;

    fn midi(self) -> Option<Self::Midi>;
    fn analog(self) -> Option<Self::Analog>;
}

impl<A, B> ValueIter for Either<A, B>
where
    A: ValueIter,
    B: ValueIter,
{
    type Midi = Either<A::Midi, B::Midi>;
    type Analog = Either<A::Analog, B::Analog>;

    fn midi(self) -> Option<Self::Midi> {
        match self {
            Self::Left(val) => val.midi().map(Either::Left),
            Self::Right(val) => val.midi().map(Either::Right),
        }
    }
    fn analog(self) -> Option<Self::Analog> {
        match self {
            Self::Left(val) => val.analog().map(Either::Left),
            Self::Right(val) => val.analog().map(Either::Right),
        }
    }
}

/// Implementation detail since Rust doesn't take associated types into account when checking whether
/// two implementations overlap.
pub trait ValueIterImplHelper<T> {
    type AnyIter: ValueIter + Send;

    fn mk_valueiter(other: T) -> Self::AnyIter;
}

pub enum NoMidi {}
pub enum NoAnalog {}

impl Iterator for NoMidi {
    type Item = MidiEventType;
    fn next(&mut self) -> Option<Self::Item> {
        unreachable!()
    }
}
impl Iterator for NoAnalog {
    type Item = I0F32;
    fn next(&mut self) -> Option<Self::Item> {
        unreachable!()
    }
}
impl ExactSizeIterator for NoMidi {
    fn len(&self) -> usize {
        unreachable!()
    }
}
impl ExactSizeIterator for NoAnalog {
    fn len(&self) -> usize {
        unreachable!()
    }
}

impl<T: ExactSizeIterator<Item = MidiEventType> + Send> ValueIterImplHelper<T> for MidiEventType {
    type AnyIter = AnyIter<T, NoAnalog>;
    fn mk_valueiter(other: T) -> Self::AnyIter {
        AnyIter::Midi(other)
    }
}

impl<T: ExactSizeIterator<Item = I0F32> + Send> ValueIterImplHelper<T> for I0F32 {
    type AnyIter = AnyIter<NoMidi, T>;

    fn mk_valueiter(other: T) -> Self::AnyIter {
        AnyIter::Analog(other)
    }
}

impl<A, B, V> From<V> for AnyIter<A, B>
where
    A: ExactSizeIterator<Item = MidiEventType> + Send,
    B: ExactSizeIterator<Item = I0F32> + Send,
    V: ExactSizeIterator,
    V::Item: ValueIterImplHelper<V, AnyIter = AnyIter<A, B>>,
{
    fn from(other: V) -> AnyIter<A, B> {
        V::Item::mk_valueiter(other)
    }
}

pub enum AnyIter<A, B> {
    Midi(A),
    Analog(B),
}

impl<A, B> ValueIter for AnyIter<A, B>
where
    A: ExactSizeIterator<Item = MidiEventType> + Send,
    B: ExactSizeIterator<Item = I0F32> + Send,
{
    type Midi = A;
    type Analog = B;

    fn midi(self) -> Option<<Self as ValueIter>::Midi> {
        match self {
            Self::Midi(inner) => Some(inner),
            Self::Analog(_) => None,
        }
    }

    fn analog(self) -> Option<<Self as ValueIter>::Analog> {
        match self {
            Self::Midi(_) => None,
            Self::Analog(inner) => Some(inner),
        }
    }
}

pub trait GetOutput: Component {
    // TODO: Use GATs to allow adapators to be used internally.
    type OutputIter: ValueIter + Send;

    fn output<Ctx>(&self, id: Self::OutputSpecifier, ctx: Ctx) -> Self::OutputIter
    where
        Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>;

    fn update<Ctx>(&mut self, _ctx: Ctx)
    where
        Ctx: GetInput<Self::InputSpecifier> + GetParam<Self::ParamSpecifier>,
    {
    }
}

pub trait Context<ISpec, PSpec>: GetInput<ISpec> + GetParam<PSpec> {
    /// Samples per second
    fn samples(&self) -> usize;
}

pub trait GetInput<Spec> {
    type Iter: ValueIter + Send;

    // `None` means that this input is not wired
    fn input(&self, spec: Spec) -> Option<Self::Iter>;
}

pub trait GetParam<Spec> {
    fn param(&self, spec: Spec) -> Value;
}

#[macro_export]
macro_rules! component_set {
    ($v:vis mod $name:ident { $($t:ident),* }) => {
        #[allow(dead_code)]
        $v mod $name {
            use $crate::GetOutput;

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

            pub enum Iter<$($t),*> {
                $( $t($t) ),*
            }

            impl<$($t),*, __V> Iterator for Iter<$($t),*>
            where $($t: ExactSizeIterator<Item = __V>),*
            {
                type Item = __V;

                fn next(&mut self) -> Option<Self::Item> {
                    match self {
                        $(
                            Self::$t(inner) => Iterator::next(inner),
                        )*
                    }
                }

                fn size_hint(&self) -> (usize, Option<usize>) {
                    (self.len(), Some(self.len()))
                }
            }

            impl<$($t),*, __V> std::iter::ExactSizeIterator for Iter<$($t),*>
            where $($t: std::iter::ExactSizeIterator<Item = __V>),*
            {
                fn len(&self) -> usize {
                    match self {
                        $(
                            Self::$t(inner) => std::iter::ExactSizeIterator::len(inner),
                        )*
                    }
                }
            }

            pub enum ValueIter<$($t),*> {
                $( $t($t) ),*
            }

            impl<$($t),*> $crate::ValueIter for ValueIter<$($t),*>
            where $($t: $crate::ValueIter),*
            {
                type Midi = Iter<$($t::Midi),*>;
                type Analog = Iter<$($t::Analog),*>;

                fn midi(self) -> Option<Self::Midi> {
                    match self {
                        $(
                            Self::$t(inner) => inner.midi().map(Iter::$t),
                        )*
                    }
                }

                fn analog(self) -> Option<Self::Analog> {
                    match self {
                        $(
                            Self::$t(inner) => inner.analog().map(Iter::$t),
                        )*
                    }
                }
            }

            $(
                impl From<super::$t> for Component {
                    fn from(other: super::$t) -> Self {
                        Component::$t(other)
                    }
                }

                #[allow(unreachable_code)]
                impl From<<super::$t as $crate::Component>::InputSpecifier> for Input {
                    fn from(other: <super::$t as $crate::Component>::InputSpecifier) -> Self {
                        Self::$t(other)
                    }
                }

                #[allow(unreachable_code)]
                impl From<<super::$t as $crate::Component>::OutputSpecifier> for Output {
                    fn from(other: <super::$t as $crate::Component>::OutputSpecifier) -> Self {
                        Self::$t(other)
                    }
                }

                #[allow(unreachable_code)]
                impl From<<super::$t as $crate::Component>::ParamSpecifier> for Param {
                    fn from(other: <super::$t as $crate::Component>::ParamSpecifier) -> Self {
                        Self::$t(other)
                    }
                }
            )*

            enum ParamDefaultsInner {
                $($t(&'static [<super::$t as $crate::Component>::ParamSpecifier])),*
            }

            pub struct ParamDefaults {
                inner: ParamDefaultsInner,
            }

            impl Iterator for ParamDefaults
            where
            $(
                <super::$t as $crate::Component>::ParamSpecifier: $crate::Param
            ),*
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

            impl ExactSizeIterator for ParamDefaults {
                fn len(&self) -> usize {
                    match &self.inner {
                        $(
                            ParamDefaultsInner::$t(inner) => {
                                inner.len()
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

                #[allow(unreachable_code)]
                fn update<Ctx>(&mut self, ctx: Ctx)
                where
                    Ctx: $crate::GetInput<$crate::AnyInputSpec> + $crate::GetParam<$crate::AnyParamSpec>
                {
                    match self {
                        $(
                            Self::$t(val) => {
                                use $crate::Specifier;

                                val.update(
                                    $crate::component::QuickContext::new(
                                        ctx,
                                        |ctx: &Ctx, spec: <super::$t as $crate::Component>::InputSpecifier| {
                                            ctx.input($crate::AnyInputSpec(spec.id()))
                                        },
                                        |ctx: &Ctx, spec: <super::$t as $crate::Component>::ParamSpecifier| ctx.param($crate::AnyParamSpec(spec.id())),
                                    )
                                )
                            },
                        )*
                    }
                }
            }

            impl $crate::ComponentSetOut for Component
            where
                $( super::$t: $crate::GetOutput ),*
            {
                type OutputIter = ValueIter<$(<super::$t as $crate::GetOutput>::OutputIter),*>;

                #[allow(unreachable_code)]
                fn output<Ctx>(&self, id: $crate::AnyOutputSpec, ctx: Ctx) -> Self::OutputIter
                where
                    Ctx: $crate::GetInput<$crate::AnyInputSpec> + $crate::GetParam<$crate::AnyParamSpec>
                {
                    match self {
                        $(
                            Self::$t(val) => {
                                use $crate::Specifier;

                                ValueIter::$t(val.output(
                                    <<super::$t as $crate::Component>::OutputSpecifier as $crate::Specifier>::from_id(id.0),
                                    $crate::component::QuickContext::new(
                                        ctx,
                                        |ctx: &Ctx, spec: <super::$t as $crate::Component>::InputSpecifier| {
                                            ctx.input($crate::AnyInputSpec(spec.id()))
                                        },
                                        |ctx: &Ctx, spec: <super::$t as $crate::Component>::ParamSpecifier| ctx.param($crate::AnyParamSpec(spec.id())),
                                    )
                                ))
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

    type ParamDefaults: ExactSizeIterator<Item = Value> + Send;

    fn types(&self) -> Types;

    fn param_defaults(&self) -> Self::ParamDefaults;

    // TODO: Support MIDI
    fn update<Ctx>(&mut self, ctx: Ctx)
    where
        Ctx: GetInput<AnyInputSpec> + GetParam<AnyParamSpec>;
}

pub trait ComponentSetOut: ComponentSet {
    type OutputIter: ValueIter + Send;

    fn output<Ctx>(&self, id: AnyOutputSpec, ctx: Ctx) -> Self::OutputIter
    where
        Ctx: GetInput<AnyInputSpec> + GetParam<AnyParamSpec>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextMeta {
    pub samples: usize,
}

pub struct QuickContext<C, InputFn, ParamFn> {
    ctx: C,
    input_fn: InputFn,
    param_fn: ParamFn,
}

impl<InputFn> QuickContext<(), InputFn, ()> {
    pub fn input(input_fn: InputFn) -> Self {
        Self::new((), input_fn, ())
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

// TODO: Support MIDI inputs
impl<C, InputFn, ParamFn, Spec, I> GetInput<Spec> for QuickContext<C, InputFn, ParamFn>
where
    InputFn: Fn(&C, Spec) -> Option<I>,
    I: ValueIter + Send,
{
    type Iter = I;

    fn input(&self, spec: Spec) -> Option<Self::Iter> {
        (self.input_fn)(&self.ctx, spec)
    }
}

impl<C, InputFn, ParamFn, Spec> GetParam<Spec> for QuickContext<C, InputFn, ParamFn>
where
    ParamFn: Fn(&C, Spec) -> Value,
{
    fn param(&self, spec: Spec) -> Value {
        (self.param_fn)(&self.ctx, spec)
    }
}

impl<C, Spec> GetInput<Spec> for &'_ C
where
    C: GetInput<Spec>,
{
    type Iter = C::Iter;

    fn input(&self, spec: Spec) -> Option<Self::Iter> {
        C::input(*self, spec)
    }
}

impl<C, Spec> GetParam<Spec> for &'_ C
where
    C: GetParam<Spec>,
{
    fn param(&self, spec: Spec) -> Value {
        C::param(*self, spec)
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

mod marker {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct Param;
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct Input;
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct Output;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct GenericWire<Marker, Id> {
    io_index: SpecId,
    element: ElementSpecifier<Id>,
    _marker: PhantomData<Marker>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Wire<Marker>(GenericWire<Marker, ComponentId>);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct WireSrc(GenericWire<marker::Output, ()>);

#[derive(Debug, Clone, PartialEq)]
enum WireDstInner {
    Param(Value, GenericWire<marker::Param, ()>),
    Input(GenericWire<marker::Input, ()>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WireDst(WireDstInner);

impl WireDst {
    pub fn rack_output<S: Specifier>(output: S) -> Self {
        WireDst(WireDstInner::Input(GenericWire {
            io_index: output.id(),
            element: ElementSpecifier::Rack,
            _marker: PhantomData,
        }))
    }

    pub fn component_input<S: Specifier>(component_index: usize, input: S) -> Self {
        WireDst(WireDstInner::Input(GenericWire {
            io_index: input.id(),
            element: ElementSpecifier::Component {
                id: (),
                index: component_index,
            },
            _marker: PhantomData,
        }))
    }

    pub fn component_param<S: Specifier, V: Into<Value>>(
        component_index: usize,
        param: S,
        value: V,
    ) -> Self {
        let value = value.into();
        WireDst(WireDstInner::Param(
            value,
            GenericWire {
                io_index: param.id(),
                element: ElementSpecifier::Component {
                    id: (),
                    index: component_index,
                },
                _marker: PhantomData,
            },
        ))
    }
}

impl WireSrc {
    fn fill_id(self, f: impl FnOnce(usize) -> ComponentId) -> Wire<marker::Output> {
        Wire(GenericWire {
            io_index: self.0.io_index,
            element: self.0.element.fill_id(f),
            _marker: PhantomData,
        })
    }

    pub fn rack_input<S: Specifier>(input: S) -> Self {
        WireSrc(GenericWire {
            io_index: input.id(),
            element: ElementSpecifier::Rack,
            _marker: PhantomData,
        })
    }

    pub fn component_output<S: Specifier>(component_index: usize, output: S) -> Self {
        WireSrc(GenericWire {
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

impl<Id> GenericWire<marker::Input, Id> {
    fn input_id(&self) -> AnyInputSpec {
        AnyInputSpec(self.io_index)
    }
}

impl<Id> GenericWire<marker::Param, Id> {
    fn param_id(&self) -> AnyInputSpec {
        AnyInputSpec(self.io_index)
    }
}

impl<Id> GenericWire<marker::Output, Id> {
    fn output_id(&self) -> AnyOutputSpec {
        AnyOutputSpec(self.io_index)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ComponentId(usize);

#[derive(Debug, Clone, PartialEq)]
struct ParamWire {
    src: Wire<marker::Output>,
    value: Value,
}

// TODO: Scenes
#[derive(Debug, Clone, PartialEq)]
struct ParamValue {
    natural_value: Value,
    wire: Option<ParamWire>,
}

#[derive(Debug, Clone, PartialEq)]
struct TaggedComponent<C> {
    id: ComponentId,
    inner: C,
    params: Vec<ParamValue>,
    wires: Vec<Option<Wire<marker::Output>>>,
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
    components: Vec<TaggedComponent<C>>,
    out_wires: Vec<Option<Wire<marker::Output>>>,
    _marker: PhantomData<(InputSpec, OutputSpec)>,
}

impl<C, InputSpec, OutputSpec> Rack<C, InputSpec, OutputSpec>
where
    InputSpec: Specifier,
    OutputSpec: Specifier,
    C: ComponentSetOut,
{
    pub fn new() -> Self {
        use std::iter;

        Rack {
            ids: ComponentIdGen::new(),
            components: vec![],
            out_wires: iter::repeat(None).take(OutputSpec::TYPES.len()).collect(),
            _marker: PhantomData,
        }
    }

    pub fn update<Ctx>(&mut self, ctx: Ctx)
    where
        Ctx: GetInput<InputSpec>,
    {
        self.as_slice().update(SimpleCow::Owned(ctx));
    }

    // TODO: Return a result
    pub fn wire(&mut self, src: WireSrc, dst: WireDst) {
        let filled_output = src.fill_id(|index| self.components[index].id);
        match dst.0 {
            WireDstInner::Input(dst) => match dst.element() {
                ElementSpecifier::Component { id: (), index } => {
                    self.components[index].wires[dst.input_id().0] = Some(filled_output);
                }
                ElementSpecifier::Rack => self.out_wires[dst.input_id().0] = Some(filled_output),
            },
            WireDstInner::Param(val, dst) => match dst.element() {
                ElementSpecifier::Component { id: (), index } => {
                    self.components[index].params[dst.param_id().0].wire = Some(ParamWire {
                        value: val,
                        src: filled_output,
                    })
                }
                ElementSpecifier::Rack => unimplemented!(),
            },
        }
    }

    pub fn set_param<S: Specifier, V: Into<Value>>(
        &mut self,
        component: usize,
        param: S,
        value: V,
    ) {
        self.components[component].params[param.id()].natural_value = value.into();
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
            params: params
                .map(|def| ParamValue {
                    natural_value: def,
                    wire: None,
                })
                .collect(),
        });

        out
    }

    /// Get a specific output of this rack. This takes a mutable reference because it technically
    /// isn't pure, but it _is_ idempotent.
    pub fn output<'a, Ctx: 'a>(
        &mut self,
        spec: OutputSpec,
        ctx: Ctx,
    ) -> Option<impl ValueIter + Send + 'a>
    where
        Ctx: GetInput<InputSpec>,
        C: 'a,
    {
        let wire = self.out_wires[spec.id()]?;

        let out = self.as_slice().as_ref().output(wire, &ctx);

        out
    }

    fn as_slice(&mut self) -> RackSlice<&'_ mut [TaggedComponent<C>], InputSpec> {
        RackSlice {
            components: &mut self.components,
            _marker: PhantomData::<InputSpec>,
        }
    }
}

// At one point we cached intermediate values, but I think that the
struct RackSlice<Components, InputSpec> {
    components: Components,
    _marker: PhantomData<InputSpec>,
}

fn get_input<'inner, 'outer, InputSpec, Ctx, C>(
    (ctx, wires, rest, _): &'inner (
        &'outer Ctx,
        &'outer [Option<Wire<marker::Output>>],
        &'outer [TaggedComponent<C>],
        &'outer [ParamValue],
    ),
    spec: AnyInputSpec,
) -> Option<impl ValueIter + Send + 'outer>
where
    InputSpec: Specifier,
    C: ComponentSetOut,
    C::OutputIter: 'outer,
    Ctx: GetInput<InputSpec>,
{
    let wire = wires[spec.0]?;

    // TODO: When generic associated types are implemented we can remove
    //       this allocation
    Some(
        RackSlice {
            components: *rest,
            _marker: PhantomData,
        }
        .output(wire, *ctx)?,
    )
}

fn get_param<'inner, 'outer, InputSpec, Ctx, C>(
    (ctx, _, rest, params): &'inner (
        &'outer Ctx,
        &'outer [Option<Wire<marker::Output>>],
        &'outer [TaggedComponent<C>],
        &'outer [ParamValue],
    ),
    spec: AnyParamSpec,
) -> Value
where
    InputSpec: Specifier,
    C: ComponentSetOut,
    C::OutputIter: 'outer,
    Ctx: GetInput<InputSpec>,
{
    use fixed::FixedU32;

    type UCont = FixedU32<consts::U32>;

    let ParamValue {
        natural_value: nat_val,
        wire,
    } = &params[spec.0];

    wire.as_ref()
        .and_then(|ParamWire { src, value }| {
            RackSlice {
                components: *rest,
                _marker: PhantomData,
            }
            .output(*src, *ctx)
            .map(|outputs| (value, outputs))
        })
        .map(|(wire_value, outputs)| {
            let outputs = outputs.analog().unwrap();
            let wire_value = s_to_u(*wire_value);
            let len = outputs.len() as u32;
            let average_output_this_tick: UCont = outputs.map(|o| s_to_u(o) / len).sum();
            let unat = s_to_u(*nat_val);

            // Weighted average: wire value == max means out is `average_output_this_tick`,
            // wire value == min means out is `unat`, and values between those extremes lerp
            // between the two.
            u_to_s(
                (unat * (UCont::max_value() - average_output_this_tick))
                    + wire_value * average_output_this_tick,
            )
        })
        .unwrap_or(*nat_val)
}

impl<C, InputSpec> RackSlice<&'_ mut [TaggedComponent<C>], InputSpec>
where
    C: ComponentSetOut,
    InputSpec: Specifier,
{
    fn update<Ctx>(&mut self, ctx: SimpleCow<'_, Ctx>)
    where
        Ctx: GetInput<InputSpec>,
    {
        for index in (0..self.components.len()).rev() {
            let (rest, this) = self.components.split_at_mut(index);
            let this = &mut this[0];

            let inner = &mut this.inner;
            let wires = &this.wires[..];
            let params = &this.params[..];

            inner.update(QuickContext::new(
                (ctx.as_ref(), wires, &*rest, params),
                get_input::<InputSpec, Ctx, C>,
                get_param::<InputSpec, Ctx, C>,
            ));
        }
    }

    fn as_ref(&self) -> RackSlice<&'_ [TaggedComponent<C>], InputSpec> {
        RackSlice {
            components: &*self.components,
            _marker: PhantomData,
        }
    }
}

enum SimpleCow<'a, C> {
    Borrowed(&'a C),
    Owned(C),
}

impl<'a, C> SimpleCow<'a, C> {
    fn as_ref(&self) -> &C {
        match self {
            Self::Borrowed(v) => v,
            Self::Owned(v) => v,
        }
    }
}

impl<C> From<C> for SimpleCow<'static, C> {
    fn from(other: C) -> Self {
        SimpleCow::Owned(other)
    }
}

impl<'a, C> From<&'a C> for SimpleCow<'a, C> {
    fn from(other: &'a C) -> Self {
        SimpleCow::Borrowed(other)
    }
}

impl<C, InputSpec> RackSlice<&'_ [TaggedComponent<C>], InputSpec>
where
    C: ComponentSetOut,
    InputSpec: Specifier,
{
    fn output<Ctx>(
        &self,
        Wire(wire): Wire<marker::Output>,
        ctx: &Ctx,
    ) -> Option<impl ValueIter + Send>
    where
        Ctx: GetInput<InputSpec>,
    {
        match wire.element() {
            ElementSpecifier::Component { id, index } => {
                let (rest, this) = self.components.split_at(index);
                // TODO: We will never allow backwards- or self-wiring, so maybe we can have some
                //       safe abstraction that allows us to omit checks in the `split_at_mut` and
                //       here.
                let this = &this[0];

                if this.id != id {
                    // TODO: Disconnect this wire too?
                    return None;
                }

                let inner = &this.inner;
                let wires = &this.wires[..];
                let params = &this.params[..];

                let out = inner.output(
                    AnyOutputSpec(wire.output_id().0),
                    QuickContext::new(
                        (ctx, wires, rest, params),
                        get_input::<InputSpec, Ctx, C>,
                        get_param::<InputSpec, Ctx, C>,
                    ),
                );

                Some(Either::Left(out))
            }
            ElementSpecifier::Rack => ctx
                .input(InputSpec::from_id(wire.io_index))
                .map(Either::Right),
        }
    }
}
