use crate::{
    AnyComponent, AnyInputSpec, AnyOutputSpec, AnyParamSpec, GetInput, QuickContext, SpecId,
    Specifier, Value, ValueExt, ValueIter,
};
use fixed::types::U1F31;
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
pub struct WireSrc(GenericWire<marker::Output, usize>);

#[derive(Debug, Clone, PartialEq)]
enum WireDstInner {
    Param(Value, GenericWire<marker::Param, usize>),
    Input(GenericWire<marker::Input, usize>),
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
                id: component_index,
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
    inner: C,
    params: Vec<ParamValue>,
    wires: Vec<Option<Wire<marker::Output>>>,
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

#[derive(Debug, Clone, PartialEq)]
struct ComponentVec<C> {
    ids: ComponentIdGen,
    storage: FxHashMap<ComponentId, TaggedComponent<C>>,
    indices: Vec<ComponentId>,
}

impl<C> Default for ComponentVec<C> {
    fn default() -> Self {
        Self {
            ids: Default::default(),
            storage: Default::default(),
            indices: Default::default(),
        }
    }
}

impl<C> ComponentVec<C> {
    fn push(&mut self, new: TaggedComponent<C>) -> ComponentId {
        let new_id = self.ids.next();
        self.storage.insert(new_id, new);
        self.indices.push(new_id);
        new_id
    }

    fn get_id(&self, index: usize) -> ComponentId {
        self.indices[index]
    }

    fn len(&self) -> usize {
        self.indices.len()
    }
}

impl<C> Index<usize> for ComponentVec<C> {
    type Output = TaggedComponent<C>;

    fn index(&self, i: usize) -> &Self::Output {
        &self[self.get_id(i)]
    }
}

impl<C> IndexMut<usize> for ComponentVec<C> {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        let id = self.get_id(i);
        &mut self[id]
    }
}

impl<C> Index<ComponentId> for ComponentVec<C> {
    type Output = TaggedComponent<C>;

    fn index(&self, i: ComponentId) -> &Self::Output {
        &self.storage[&i]
    }
}

impl<C> IndexMut<ComponentId> for ComponentVec<C> {
    fn index_mut(&mut self, i: ComponentId) -> &mut Self::Output {
        self.storage.get_mut(&i).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rack<C, InputSpec, OutputSpec> {
    components: ComponentVec<C>,
    out_wires: Vec<Option<Wire<marker::Output>>>,
    _marker: PhantomData<(InputSpec, OutputSpec)>,
}

impl<C, InputSpec, OutputSpec> Rack<C, InputSpec, OutputSpec>
where
    InputSpec: Specifier,
    OutputSpec: Specifier,
    C: AnyComponent,
{
    pub fn new() -> Self {
        use std::iter;

        Rack {
            components: Default::default(),
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
        let filled_output = src.fill_id(|index| self.components.get_id(index));
        match dst.0 {
            WireDstInner::Input(dst) => match dst.element() {
                ElementSpecifier::Component { id } => {
                    self.components[id].wires[dst.input_id().0] = Some(filled_output);
                }
                ElementSpecifier::Rack => self.out_wires[dst.input_id().0] = Some(filled_output),
            },
            WireDstInner::Param(val, dst) => match dst.element() {
                ElementSpecifier::Component { id } => {
                    self.components[id].params[dst.param_id().0].wire = Some(ParamWire {
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
    (ctx, wires, rest, _): &'inner (
        &'outer Ctx,
        &'outer [Option<Wire<marker::Output>>],
        &'outer ComponentVec<C>,
        &'outer [ParamValue],
    ),
    spec: AnyInputSpec,
) -> Option<impl ValueIter + Send + 'outer>
where
    InputSpec: Specifier,
    C: AnyComponent,
    C::OutputIter: 'outer,
    Ctx: GetInput<InputSpec>,
{
    let wire = wires[spec.0]?;

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
        &'outer ComponentVec<C>,
        &'outer [ParamValue],
    ),
    spec: AnyParamSpec,
) -> Value
where
    InputSpec: Specifier,
    C: AnyComponent,
    C::OutputIter: 'outer,
    Ctx: GetInput<InputSpec>,
{
    type UCont = U1F31;

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
            let wire_value = (*wire_value).to_u();
            let len = outputs.len() as u32;
            let average_output_this_tick: UCont = outputs.map(|o| o.to_u() / len).sum();
            let unat = (*nat_val).to_u();

            // Weighted average: wire value == max means out is `average_output_this_tick`,
            // wire value == min means out is `unat`, and values between those extremes lerp
            // between the two.
            Value::from_u(
                (unat * (UCont::max_value() - average_output_this_tick))
                    + wire_value * average_output_this_tick,
            )
        })
        .unwrap_or(*nat_val)
}

impl<C, InputSpec> RackSlice<&'_ mut ComponentVec<C>, InputSpec>
where
    C: AnyComponent,
    InputSpec: Specifier,
{
    fn update<Ctx>(&mut self, ctx: SimpleCow<'_, Ctx>)
    where
        Ctx: GetInput<InputSpec>,
    {
        for i in 0..self.components.len() {
            let new = {
                let this = &self.components[i];
                let inner = &this.inner;
                let wires = &this.wires[..];
                let params = &this.params[..];

                inner.update(QuickContext::new(
                    (ctx.as_ref(), wires, &*self.components, params),
                    get_input::<InputSpec, Ctx, C>,
                    get_param::<InputSpec, Ctx, C>,
                ))
            };

            self.components[i].inner = new;
        }
    }

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
            ElementSpecifier::Component { id } => {
                // TODO: We will never allow backwards- or self-wiring, so maybe we can have some
                //       safe abstraction that allows us to omit checks in the `split_at_mut` and
                //       here.
                let this = &self.components[id];

                let inner = &this.inner;
                let wires = &this.wires[..];
                let params = &this.params[..];

                let out = inner.output(
                    AnyOutputSpec(wire.output_id().0),
                    QuickContext::new(
                        (ctx, wires, self.components, params),
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
