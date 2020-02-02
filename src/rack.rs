use crate::{
    components::{
        anycomponent::{AnyContext, AnyUiElement},
        PossiblyEither,
    },
    context::{ContextMeta, GetGlobalInput},
    params::{HasStorage, ParamStorage, Storage},
    AnyComponent, AnyInputSpec, AnyOutputSpec, AnyParamSpec, RuntimeSpecifier, SpecId, Value,
};
use fxhash::FxHashMap;
use std::{
    any::Any,
    fmt,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

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
    fn param_id(&self) -> AnyParamSpec {
        AnyParamSpec(self.io_index)
    }
}

impl<Id> GenericWire<marker::Output, Id> {
    fn output_id(&self) -> AnyOutputSpec {
        AnyOutputSpec(self.io_index)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ComponentId(usize);

impl fmt::Display for ComponentId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamWire {
    pub src: Wire<marker::Output>,
    pub value: Value,
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
    inputs: C::InputStorage,
}

impl<C> std::fmt::Debug for TaggedComponent<C>
where
    C: AnyComponent + std::fmt::Debug,
    C::ParamStorage: std::fmt::Debug,
    C::InputStorage: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("TaggedComponent")
            .field("inner", &self.inner)
            .field("params", &self.params)
            .field("inputs", &self.inputs)
            .finish()
    }
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

impl<'a, C> IntoIterator for &'a ComponentVec<C>
where
    C: AnyComponent + 'a,
{
    type Item = (ComponentId, &'a TaggedComponent<C>);
    type IntoIter = impl ExactSizeIterator<Item = Self::Item> + 'a;

    fn into_iter(self) -> Self::IntoIter {
        self.indices.iter().map(move |i| (*i, &self.storage[i]))
    }
}

impl<C> std::fmt::Debug for ComponentVec<C>
where
    C: AnyComponent + std::fmt::Debug,
    C::ParamStorage: std::fmt::Debug,
    C::InputStorage: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("TaggedComponent")
            .field("ids", &self.ids)
            .field("storage", &self.storage)
            .field("indices", &self.indices)
            .finish()
    }
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
    OutputSpec: HasStorage<InternalWire>,
{
    components: ComponentVec<C>,
    out_wires: OutputSpec::Storage,
    _marker: PhantomData<(InputSpec, OutputSpec)>,
}

impl<C, I, O> std::fmt::Debug for Rack<C, I, O>
where
    C: AnyComponent + std::fmt::Debug,
    O: HasStorage<InternalWire>,
    O::Storage: std::fmt::Debug,
    C::ParamStorage: std::fmt::Debug,
    C::InputStorage: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Rack")
            .field("components", &self.components)
            .field("out_wires", &self.out_wires)
            .finish()
    }
}

impl<C, InputSpec, OutputSpec> Rack<C, InputSpec, OutputSpec>
where
    OutputSpec: HasStorage<InternalWire>,
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

impl<C, InputSpec, OutputSpec> fmt::Display for Rack<C, InputSpec, OutputSpec>
where
    InputSpec: RuntimeSpecifier + fmt::Display,
    OutputSpec: RuntimeSpecifier + HasStorage<InternalWire> + fmt::Display + Clone,
    C: AnyComponent,
    for<'any> &'any C: AnyUiElement<'any>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn printvalue(val: &dyn Any, f: &mut fmt::Formatter) -> fmt::Result {
            if let Some(value) = val.downcast_ref::<Value>() {
                write!(f, "{}", value)
            } else {
                Err(fmt::Error)
            }
        }

        write!(f, "def main(")?;
        let mut ispeciter = InputSpec::VALUES.iter();

        if let Some(i) = ispeciter.next() {
            write!(f, "{}", i)?;
        }

        for i in ispeciter {
            write!(f, ", {}", i)?;
        }

        writeln!(f, "):")?;

        for (i, component) in &self.components {
            write!(f, "    {}: {{", i)?;
            let mut onameiter = component.inner.output_names();

            if let Some(o) = onameiter.next() {
                write!(f, " {}", o)?;
            }

            for o in onameiter {
                write!(f, ", {}", o)?;
            }

            writeln!(f, " }} = {} {{", component.inner.name())?;

            for (_i, p) in component.inner.param_names().enumerate() {
                writeln!(f, "        {} = {{TODO}},", p)?;
            }

            let inameiter = component.inner.input_names().enumerate();

            if !inameiter.is_empty() {
                writeln!(f)?;
            }

            for (i, input) in inameiter {
                let wire = component.inputs.get(AnyInputSpec(i));

                write!(f, "        {} = ", input)?;
                match wire {
                    None => write!(f, "NONE")?,
                    Some(Wire(GenericWire {
                        io_index,
                        element: ElementSpecifier::Rack,
                        ..
                    })) => write!(f, "{}", InputSpec::VALUES[*io_index])?,
                    Some(Wire(GenericWire {
                        io_index,
                        element: ElementSpecifier::Component { id },
                        ..
                    })) => write!(
                        f,
                        "{}->{}",
                        id,
                        self.components[*id]
                            .inner
                            .input_names()
                            .nth(*io_index)
                            .unwrap()
                    )?,
                }

                writeln!(f, ",")?;
            }

            writeln!(f, "    }}")?;
        }

        writeln!(f)?;

        writeln!(f, "    return {{")?;

        for o in OutputSpec::VALUES {
            let wire = self.out_wires.get(o.clone());

            write!(f, "        {} = ", o)?;
            match wire {
                None => write!(f, "NONE")?,
                Some(Wire(GenericWire {
                    io_index,
                    element: ElementSpecifier::Rack,
                    ..
                })) => write!(f, "{}", InputSpec::VALUES[*io_index])?,
                Some(Wire(GenericWire {
                    io_index,
                    element: ElementSpecifier::Component { id },
                    ..
                })) => write!(
                    f,
                    "{}->{}",
                    id,
                    self.components[*id]
                        .inner
                        .output_names()
                        .nth(*io_index)
                        .unwrap()
                )?,
            }

            writeln!(f, ",")?;
        }

        writeln!(f, "    }}")?;

        Ok(())
    }
}

impl<C, InputSpec, OutputSpec> Rack<C, InputSpec, OutputSpec>
where
    InputSpec: RuntimeSpecifier,
    OutputSpec: RuntimeSpecifier + HasStorage<InternalWire>,
    C: AnyComponent,
{
    pub fn update<Ctx>(&mut self, ctx: &Ctx)
    where
        Ctx: GetGlobalInput<InputSpec> + ContextMeta,
    {
        self.as_mut_slice().update(ctx);
    }

    // TODO: Return a result
    pub fn wire(&mut self, src: WireSrc, dst: WireDst) {
        let filled_output = src.fill_id(|index| self.components.ids()[index]);
        match dst.0 {
            WireDstInner::Input(dst) => match dst.element() {
                ElementSpecifier::Component { id } => {
                    *self.components[id].inputs.get_mut(dst.input_id()) = Some(filled_output);
                }
                ElementSpecifier::Rack => {
                    *self
                        .out_wires
                        .get_mut(OutputSpec::from_id(dst.input_id().0)) = Some(filled_output)
                }
            },
            WireDstInner::Param(val, dst) => match dst.element() {
                ElementSpecifier::Component { id } => {
                    *self.components[id]
                        .params
                        .get_mut(dst.param_id())
                        .1
                        .downcast_mut::<InternalParamWire>()
                        .unwrap() = Some(ParamWire {
                        value: val,
                        src: filled_output,
                    })
                }
                ElementSpecifier::Rack => unimplemented!(),
            },
        }
    }

    pub fn set_param<S: RuntimeSpecifier, V: 'static>(
        &mut self,
        component: usize,
        param: S,
        value: V,
    ) {
        let (v, _) = self.components[component]
            .params
            .get_mut(AnyParamSpec(param.id()));
        let val = v.downcast_mut::<V>().expect("Incorrect param type");
        *val = value;
    }

    pub fn new_component(&mut self, component: impl Into<C>) -> usize {
        let component = component.into();
        let out = self.components.len();
        let params = component.param_default();
        let inputs = component.input_default();

        self.components.push(TaggedComponent {
            inner: component,
            inputs,
            params,
        });

        out
    }

    /// Get a specific output of this rack.
    pub fn output<'a, Ctx: 'a>(
        &'a self,
        spec: OutputSpec,
        ctx: &'a Ctx,
    ) -> Option<PossiblyEither<C::OutputIter, Ctx::Iter>>
    where
        Ctx: GetGlobalInput<InputSpec> + ContextMeta,
    {
        let wire = (*self.out_wires.get(spec))?;

        let out = self.as_slice().output(wire, ctx);

        Some(out)
    }

    fn as_slice(&self) -> RackSlice<&'_ ComponentVec<C>, InputSpec> {
        RackSlice {
            components: &self.components,
            _marker: PhantomData::<InputSpec>,
        }
    }

    fn as_mut_slice(&mut self) -> RackSlice<&'_ mut ComponentVec<C>, InputSpec> {
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

impl<C, I> Clone for RackSlice<C, I>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            components: self.components.clone(),
            _marker: PhantomData,
        }
    }
}

pub struct RackContext<'a, Ctx, C, I>
where
    C: AnyComponent,
{
    ctx: &'a Ctx,
    next: RackSlice<&'a ComponentVec<C>, I>,
    cur: &'a TaggedComponent<C>,
}

impl<'a, Ctx, C, I> ContextMeta for RackContext<'a, Ctx, C, I>
where
    C: AnyComponent,
    Ctx: ContextMeta,
{
    fn sample_rate(&self) -> u32 {
        self.ctx.sample_rate()
    }
}

impl<'a, Ctx, C, I> AnyContext for RackContext<'a, Ctx, C, I>
where
    C: AnyComponent,
    I: RuntimeSpecifier,
    Ctx: GetGlobalInput<I> + ContextMeta,
{
    type ParamStorage = C::ParamStorage;
    type InputStorage = C::InputStorage;
    type Iter = PossiblyEither<C::OutputIter, Ctx::Iter>;

    fn params(&self) -> &Self::ParamStorage {
        &self.cur.params
    }

    fn inputs(&self) -> &Self::InputStorage {
        &self.cur.inputs
    }

    fn read_wire(&self, wire: Wire<marker::Output>) -> Self::Iter {
        self.next.output(wire, self.ctx)
    }
}

impl<C, InputSpec> RackSlice<&'_ mut ComponentVec<C>, InputSpec>
where
    C: AnyComponent,
    InputSpec: RuntimeSpecifier,
{
    fn update<Ctx>(&mut self, ctx: &Ctx)
    where
        Ctx: GetGlobalInput<InputSpec> + ContextMeta,
    {
        for i in 0..self.components.len() {
            let new = {
                let cur = &self.components[i];

                cur.inner.update(&RackContext {
                    ctx,
                    next: RackSlice {
                        components: &*self.components,
                        _marker: PhantomData::<InputSpec>,
                    },
                    cur,
                })
            };

            self.components[i].inner = new;
        }
    }
}

impl<C, InputSpec> RackSlice<&'_ ComponentVec<C>, InputSpec>
where
    C: AnyComponent,
    InputSpec: RuntimeSpecifier,
{
    fn output<'a, Ctx>(
        &'a self,
        Wire(wire): Wire<marker::Output>,
        ctx: &'a Ctx,
    ) -> PossiblyEither<C::OutputIter, Ctx::Iter>
    where
        Ctx: GetGlobalInput<InputSpec> + ContextMeta,
    {
        match wire.element() {
            ElementSpecifier::Component { id } => {
                // TODO: We will never allow backwards- or self-wiring, so maybe we can have some
                //       safe abstraction that allows us to omit checks in the `split_at_mut` and
                //       here.
                let cur = &self.components[id];

                let out = cur.inner.output(
                    AnyOutputSpec(wire.output_id().0),
                    &RackContext {
                        ctx,
                        next: self.clone(),
                        cur,
                    },
                );

                PossiblyEither::Left(out)
            }
            ElementSpecifier::Rack => ctx
                .input(InputSpec::from_id(wire.io_index))
                .map(PossiblyEither::Right)
                .unwrap(),
        }
    }
}
