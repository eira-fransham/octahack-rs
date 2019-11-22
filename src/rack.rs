use crate::{
    params::{GenericSpecifier, GenericStorage, ParamStorage, Possibly},
    AnyComponent, AnyInputSpec, AnyOutputSpec, AnyParamSpec, GetInput, QuickContext,
    RuntimeSpecifier, SpecId, Value, ValueExt, ValueIter,
};
use arrayvec::ArrayVec;
use fixed::types::{U0F32, U1F31};
use fxhash::FxHashMap;
use itertools::Either;
use std::{
    iter::ExactSizeIterator,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextMeta {
    pub samples: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ElementSpecifier<Id> {
    Component { id: Id },
    Rack,
}

impl<Id> ElementSpecifier<Id> {
    fn fill_id<NewId>(self, f: impl FnOnce(Id) -> NewId) -> ElementSpecifier<NewId> {
        match self {
            Self::Component { id } => ElementSpecifier::Component { id: f(id) },
            Self::Rack => ElementSpecifier::Rack,
        }
    }
}

pub mod marker {
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

pub type InternalWire = Option<Wire<marker::Output>>;
pub type InternalParamWire = Option<ParamWire>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct WireSrc(GenericWire<marker::Output, usize>);

#[derive(Debug, Clone, PartialEq)]
enum WireDstInner {
    Param(Value, GenericWire<marker::Param, usize>),
    Input(GenericWire<marker::Input, usize>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WireDst(WireDstInner);

impl WireDst {
    pub fn rack_output<S: RuntimeSpecifier>(output: S) -> Self {
        WireDst(WireDstInner::Input(GenericWire {
            io_index: output.id(),
            element: ElementSpecifier::Rack,
            _marker: PhantomData,
        }))
    }

    pub fn component_input<S: RuntimeSpecifier>(component_index: usize, input: S) -> Self {
        WireDst(WireDstInner::Input(GenericWire {
            io_index: input.id(),
            element: ElementSpecifier::Component {
                id: component_index,
            },
            _marker: PhantomData,
        }))
    }

    pub fn component_param<S: RuntimeSpecifier, V: Into<Value>>(
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
                    id: component_index,
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

    pub fn rack_input<S: RuntimeSpecifier>(input: S) -> Self {
        WireSrc(GenericWire {
            io_index: input.id(),
            element: ElementSpecifier::Rack,
            _marker: PhantomData,
        })
    }

    pub fn component_output<S: RuntimeSpecifier>(component_index: usize, output: S) -> Self {
        WireSrc(GenericWire {
            io_index: output.id(),
            element: ElementSpecifier::Component {
                id: component_index,
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
pub struct ParamWire {
    src: Wire<marker::Output>,
    value: Value,
}

// TODO: Scenes
#[derive(Debug, Clone, PartialEq)]
struct ParamValue {
    natural_value: Value,
    wire: Option<ParamWire>,
}

struct TaggedComponent<C>
where
    C: AnyComponent,
{
    inner: C,
    params: C::ParamStorage,
    param_wires: C::ParamWireStorage,
    input_wires: C::InputWireStorage,
}

#[derive(Default, Debug, Clone, PartialEq)]
struct ComponentIdGen {
    cur: usize,
}

impl ComponentIdGen {
    fn next(&mut self) -> ComponentId {
        let cur = self.cur;
        self.cur += 1;
        ComponentId(cur)
    }
}

struct ComponentVec<C>
where
    C: AnyComponent,
{
    ids: ComponentIdGen,
    storage: FxHashMap<ComponentId, TaggedComponent<C>>,
    indices: Vec<ComponentId>,
}

impl<C> Default for ComponentVec<C>
where
    C: AnyComponent,
{
    fn default() -> Self {
        Self {
            ids: Default::default(),
            storage: Default::default(),
            indices: Default::default(),
        }
    }
}

impl<C> ComponentVec<C>
where
    C: AnyComponent,
{
    fn push(&mut self, new: TaggedComponent<C>) -> ComponentId {
        let new_id = self.ids.next();
        self.storage.insert(new_id, new);
        self.indices.push(new_id);
        new_id
    }

    fn ids(&self) -> &[ComponentId] {
        &self.indices
    }

    fn ids_mut(&mut self) -> &mut [ComponentId] {
        &mut self.indices
    }

    fn len(&self) -> usize {
        self.indices.len()
    }
}

impl<C> Index<usize> for ComponentVec<C>
where
    C: AnyComponent,
{
    type Output = TaggedComponent<C>;

    fn index(&self, i: usize) -> &Self::Output {
        &self[self.ids()[i]]
    }
}

impl<C> IndexMut<usize> for ComponentVec<C>
where
    C: AnyComponent,
{
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        let id = self.ids()[i];
        &mut self[id]
    }
}

impl<C> Index<ComponentId> for ComponentVec<C>
where
    C: AnyComponent,
{
    type Output = TaggedComponent<C>;

    fn index(&self, i: ComponentId) -> &Self::Output {
        &self.storage[&i]
    }
}

impl<C> IndexMut<ComponentId> for ComponentVec<C>
where
    C: AnyComponent,
{
    fn index_mut(&mut self, i: ComponentId) -> &mut Self::Output {
        self.storage.get_mut(&i).unwrap()
    }
}

pub struct Rack<C, InputSpec, OutputSpec>
where
    C: AnyComponent,
    OutputSpec: GenericSpecifier<InternalWire>,
{
    components: ComponentVec<C>,
    // TODO: Use `GenericSpecifier::WireStorage`
    out_wires: <OutputSpec as GenericSpecifier<InternalWire>>::Storage,
    _marker: PhantomData<(InputSpec, OutputSpec)>,
}

impl<C, InputSpec, OutputSpec> Rack<C, InputSpec, OutputSpec>
where
    OutputSpec: GenericSpecifier<InternalWire>,
    OutputSpec::Storage: Default,
    C: AnyComponent,
{
    pub fn new() -> Self {
        Rack {
            components: Default::default(),
            out_wires: Default::default(),
            _marker: PhantomData,
        }
    }
}

impl<C, InputSpec, OutputSpec> Rack<C, InputSpec, OutputSpec>
where
    InputSpec: RuntimeSpecifier,
    OutputSpec: RuntimeSpecifier + GenericSpecifier<InternalWire>,
    for<'a> <<<C as AnyComponent>::ParamStorage as ParamStorage<'a>>::Refs as Iterator>::Item:
        Possibly<&'a Value>,
    C: AnyComponent,
{
    pub fn update<Ctx>(&mut self, ctx: Ctx)
    where
        Ctx: GetInput<InputSpec>,
    {
        self.as_slice().update(SimpleCow::Owned(ctx));
    }

    // TODO: Return a result
    pub fn wire(&mut self, src: WireSrc, dst: WireDst) {
        let filled_output = src.fill_id(|index| self.components.ids()[index]);
        match dst.0 {
            WireDstInner::Input(dst) => match dst.element() {
                ElementSpecifier::Component { id } => {
                    *self.components[id]
                        .input_wires
                        .refs_mut()
                        .nth(dst.input_id().0)
                        .unwrap() = Some(filled_output);
                }
                ElementSpecifier::Rack => {
                    *self.out_wires.refs_mut().nth(dst.input_id().0).unwrap() = Some(filled_output)
                }
            },
            WireDstInner::Param(val, dst) => match dst.element() {
                ElementSpecifier::Component { id } => {
                    *self.components[id]
                        .param_wires
                        .refs_mut()
                        .nth(dst.param_id().0)
                        .unwrap() = Some(ParamWire {
                        value: val,
                        src: filled_output,
                    })
                }
                ElementSpecifier::Rack => unimplemented!(),
            },
        }
    }

    pub fn set_param<'a, S: RuntimeSpecifier, V: 'a>(
        &'a mut self,
        component: usize,
        param: S,
        value: V,
    )
    where <<<C as AnyComponent>::ParamStorage as crate::params::ParamStorage<'a>>::RefsMut as Iterator>::Item: crate::params::Possibly<&'a mut V>
    {
        use crate::params::{ParamStorage, Possibly};

        let v = self.components[component]
            .params
            .refs_mut()
            .nth(param.id())
            .unwrap();
        v.when_matches(|val: &mut _| *val = value)
            .unwrap_or_else(|_| unimplemented!());
    }

    pub fn new_component(&mut self, component: impl Into<C>) -> usize {
        let component = component.into();
        let out = self.components.len();
        let params = component.param_default();
        let input_wires = component.input_wires_default();
        let param_wires = component.param_wires_default();

        self.components.push(TaggedComponent {
            inner: component,
            input_wires,
            params,
            param_wires,
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
        let wire = (*self.out_wires.refs().nth(spec.id()).unwrap())?;

        let out = self.as_slice().as_ref().output(wire, &ctx);

        out
    }

    fn as_slice(&mut self) -> RackSlice<&'_ mut ComponentVec<C>, InputSpec> {
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
    (ctx, rest, input_wires, _, _): &'inner (
        &'outer Ctx,
        &'outer ComponentVec<C>,
        &'outer C::InputWireStorage,
        &'outer C::ParamStorage,
        &'outer C::ParamWireStorage,
    ),
    spec: AnyInputSpec,
) -> Option<impl ValueIter + Send + 'outer>
where
    InputSpec: RuntimeSpecifier,
    C: AnyComponent,
    C::OutputIter: 'outer,
    for<'a> <<<C as AnyComponent>::ParamStorage as ParamStorage<'a>>::Refs as Iterator>::Item:
        Possibly<&'a Value>,
    Ctx: GetInput<InputSpec>,
{
    let wire = (*input_wires.refs().nth(spec.0).unwrap())?;

    Some(
        RackSlice {
            components: *rest,
            _marker: PhantomData,
        }
        .output(wire, *ctx)?,
    )
}

/// Improves precision (and possibly performance, too) by waiting as long as possible to do division.
/// If we overflow 36 (I believe?) bits total then it crashes, but I believe that it's OK to assume
/// that doesn't happen.
fn average_fixed<I>(iter: I) -> U0F32
where
    I: ExactSizeIterator<Item = U0F32>,
{
    let len = iter.len() as u32;

    let mut cur = ArrayVec::<[U0F32; 4]>::new();
    let mut acc = U0F32::default();

    for i in iter {
        if let Some(new) = acc.checked_add(i) {
            acc = new;
        } else {
            cur.push(acc);
            acc = i;
        }
    }

    acc / len + cur.into_iter().map(|c| c / len).sum::<U0F32>()
}

fn get_param<'inner, 'outer, InputSpec, Ctx, C>(
    (ctx, rest, _, params, param_wires): &'inner (
        &'outer Ctx,
        &'outer ComponentVec<C>,
        &'outer C::InputWireStorage,
        &'outer C::ParamStorage,
        &'outer C::ParamWireStorage,
    ),
    spec: AnyParamSpec,
) -> Value
where
    InputSpec: RuntimeSpecifier,
    C: AnyComponent,
    C::OutputIter: 'outer,
    for<'a> <<<C as AnyComponent>::ParamStorage as ParamStorage<'a>>::Refs as Iterator>::Item:
        Possibly<&'a Value>,
    Ctx: GetInput<InputSpec>,
{
    type UCont = U0F32;

    let nat_val = params
        .refs()
        .nth(spec.0)
        .unwrap()
        .when_matches(|val: &Value| val)
        .unwrap_or_else(|_| unreachable!());
    let wire = param_wires.refs().nth(spec.0).unwrap();

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
            fn remap_0_1(val: U1F31) -> U0F32 {
                U0F32::from_bits(val.to_bits())
            }

            fn remap_0_2(val: U0F32) -> U1F31 {
                U1F31::from_bits(val.to_bits())
            }

            let outputs = outputs.analog().unwrap();
            let wire_value = remap_0_1(wire_value.to_u());
            let average_output_this_tick: UCont =
                average_fixed(outputs.map(|o| remap_0_1(o.to_u())));
            let unat = remap_0_1(nat_val.to_u());

            // Weighted average: wire value == max means out is `average_output_this_tick`,
            // wire value == min means out is `unat`, and values between those extremes lerp
            // between the two.
            Value::from_u(remap_0_2(
                (unat * (UCont::max_value() - average_output_this_tick))
                    + wire_value * average_output_this_tick,
            ))
        })
        .unwrap_or(*nat_val)
}

impl<C, InputSpec> RackSlice<&'_ mut ComponentVec<C>, InputSpec>
where
    C: AnyComponent,
    for<'a> <<<C as AnyComponent>::ParamStorage as ParamStorage<'a>>::Refs as Iterator>::Item:
        Possibly<&'a Value>,
    InputSpec: RuntimeSpecifier,
{
    fn update<Ctx>(&mut self, ctx: SimpleCow<'_, Ctx>)
    where
        Ctx: GetInput<InputSpec>,
    {
        for i in 0..self.components.len() {
            let new = {
                let this = &self.components[i];

                this.inner.update(QuickContext::new(
                    (
                        ctx.as_ref(),
                        &*self.components,
                        &this.input_wires,
                        &this.params,
                        &this.param_wires,
                    ),
                    get_input::<InputSpec, Ctx, C>,
                    get_param::<InputSpec, Ctx, C>,
                ))
            };

            self.components[i].inner = new;
        }
    }
}

impl<C, InputSpec> RackSlice<&'_ mut ComponentVec<C>, InputSpec>
where
    C: AnyComponent,
{
    fn as_ref(&self) -> RackSlice<&ComponentVec<C>, InputSpec> {
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

impl<C, InputSpec> RackSlice<&'_ ComponentVec<C>, InputSpec>
where
    C: AnyComponent,
    for<'a> <<<C as AnyComponent>::ParamStorage as ParamStorage<'a>>::Refs as Iterator>::Item:
        Possibly<&'a Value>,
    InputSpec: RuntimeSpecifier,
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
            ElementSpecifier::Component { id } => {
                // TODO: We will never allow backwards- or self-wiring, so maybe we can have some
                //       safe abstraction that allows us to omit checks in the `split_at_mut` and
                //       here.
                let this = &self.components[id];

                let out = this.inner.output(
                    AnyOutputSpec(wire.output_id().0),
                    QuickContext::new(
                        (
                            ctx,
                            self.components,
                            &this.input_wires,
                            &this.params,
                            &this.param_wires,
                        ),
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
