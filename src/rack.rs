use crate::{
    components::{
        anycomponent::{AnyContext, AnyMeta, AnyUiElement, AnyUiElementDisplayParamValue},
        EnumerateValues, PossiblyEither, PossiblyIter,
    },
    context::{ContextMeta, GetFunctionParam},
    params::{EitherStorage, HasStorage, ParamStorage, Storage, StorageMut},
    AnyComponent, AnyInputSpec, AnyOutputSpec, AnyParamSpec, MidiValue, RefRuntimeSpecifier,
    RuntimeSpecifier, SpecId, Uid, UidGen, UidMap, Value, XOrHasher,
};
use std::{
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ElementSpecifier<Id> {
    Component { id: Id },
    FuncInputs,
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
pub(crate) struct GenericWire<Marker, Id> {
    pub(crate) io_index: SpecId,
    pub(crate) element: ElementSpecifier<Id>,
    _marker: PhantomData<Marker>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Wire<Marker>(pub(crate) GenericWire<Marker, ComponentId>);

pub type InternalWire = Option<WireSrc>;
pub type InternalParamWire = Option<ParamWire>;

pub type WireSrc = Wire<marker::Output>;

#[derive(Debug, Clone, PartialEq)]
enum WireDstInner {
    Param(Value, GenericWire<marker::Param, ComponentId>),
    Input(GenericWire<marker::Input, ComponentId>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WireDst(WireDstInner);

impl WireDst {
    #[inline]
    pub fn rack_output<S: RuntimeSpecifier>(output: S) -> Self {
        WireDst(WireDstInner::Input(GenericWire {
            io_index: output.id(),
            element: ElementSpecifier::FuncInputs,
            _marker: PhantomData,
        }))
    }

    #[inline]
    pub fn component_input<S: RuntimeSpecifier>(id: ComponentId, input: S) -> Self {
        WireDst(WireDstInner::Input(GenericWire {
            io_index: input.id(),
            element: ElementSpecifier::Component { id },
            _marker: PhantomData,
        }))
    }

    #[inline]
    pub fn component_param<S: RuntimeSpecifier, V: Into<Value>>(
        id: ComponentId,
        param: S,
        value: V,
    ) -> Self {
        let value = value.into();
        WireDst(WireDstInner::Param(
            value,
            GenericWire {
                io_index: param.id(),
                element: ElementSpecifier::Component { id },
                _marker: PhantomData,
            },
        ))
    }
}

impl WireSrc {
    #[inline]
    pub fn rack_input<S: RuntimeSpecifier>(input: S) -> Self {
        Wire(GenericWire {
            io_index: input.id(),
            element: ElementSpecifier::FuncInputs,
            _marker: PhantomData,
        })
    }

    #[inline]
    pub fn component_output<S: RuntimeSpecifier>(id: ComponentId, output: S) -> Self {
        Wire(GenericWire {
            io_index: output.id(),
            element: ElementSpecifier::Component { id },
            _marker: PhantomData,
        })
    }
}

impl<M, Id> GenericWire<M, Id>
where
    ElementSpecifier<Id>: Copy,
{
    #[inline]
    fn element(&self) -> ElementSpecifier<Id> {
        self.element
    }
}

impl<Id> GenericWire<marker::Input, Id> {
    #[inline]
    fn input_id(&self) -> AnyInputSpec {
        AnyInputSpec(self.io_index)
    }
}

impl<Id> GenericWire<marker::Param, Id> {
    #[inline]
    fn param_id(&self) -> AnyParamSpec {
        AnyParamSpec(self.io_index)
    }
}

impl<Id> GenericWire<marker::Output, Id> {
    #[inline]
    fn output_id(&self) -> AnyOutputSpec {
        AnyOutputSpec(self.io_index)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamWire {
    pub src: WireSrc,
    pub value: Value,
}

// TODO: Scenes
#[derive(Debug, Clone, PartialEq)]
struct ParamValue {
    natural_value: Value,
    wire: Option<ParamWire>,
}

pub struct ComponentMeta<C>
where
    C: AnyComponent,
{
    pub(crate) params: C::ParamStorage,
    pub(crate) inputs: C::InputStorage,
}

pub enum Meta<C>
where
    C: AnyComponent,
{
    Component(ComponentMeta<C>),
    Function {
        func_id: FuncId,
        // params: TODO
        inputs: <AnyInputSpec as HasStorage<InternalWire>>::Storage,
    },
}

impl<C> Meta<C>
where
    C: AnyComponent,
{
    fn inputs(&self) -> impl Storage<Specifier = AnyInputSpec, Inner = InternalWire> + '_ {
        match self {
            Self::Component(meta) => EitherStorage::Left(&meta.inputs),
            Self::Function { inputs, .. } => EitherStorage::Right(inputs),
        }
    }

    fn inputs_mut(
        &mut self,
    ) -> impl StorageMut<Specifier = AnyInputSpec, Inner = InternalWire> + '_ {
        match self {
            Self::Component(meta) => EitherStorage::Left(&mut meta.inputs),
            Self::Function { inputs, .. } => EitherStorage::Right(inputs),
        }
    }
}

impl<C> Meta<C>
where
    C: AnyComponent,
{
    fn component(&self) -> Option<&ComponentMeta<C>> {
        match self {
            Self::Component(cmeta) => Some(cmeta),
            _ => None,
        }
    }

    fn component_mut(&mut self) -> Option<&mut ComponentMeta<C>> {
        match self {
            Self::Component(cmeta) => Some(cmeta),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MapWithPathGen<M> {
    path: XOrHasher,
    map: M,
}

impl<M> MapWithPathGen<M> {
    #[inline]
    fn new(map: M) -> Self {
        Self {
            path: XOrHasher::default(),
            map,
        }
    }
}

pub type MapWithPath<'a, T> = MapWithPathGen<&'a UidMap<T>>;
pub type MapWithPathMut<'a, T> = MapWithPathGen<&'a mut UidMap<T>>;

impl<M> MapWithPathGen<M>
where
    M: Deref,
{
    #[inline]
    pub(crate) fn as_ref(&self) -> MapWithPathGen<&'_ M::Target> {
        MapWithPathGen {
            path: self.path.clone(),
            map: &*self.map,
        }
    }

    #[inline]
    fn original_map(&self) -> MapWithPathGen<&M::Target> {
        MapWithPathGen {
            path: XOrHasher::default(),
            map: &*self.map,
        }
    }
}

impl<M> MapWithPathGen<M>
where
    M: DerefMut,
{
    #[inline]
    pub(crate) fn as_mut(&mut self) -> MapWithPathGen<&'_ mut M::Target> {
        MapWithPathGen {
            path: self.path.clone(),
            map: &mut *self.map,
        }
    }
}

impl<'a, T> MapWithPath<'a, T> {
    #[inline]
    fn append_path(self, path: Uid) -> MapWithPath<'a, T> {
        use std::hash::Hash;

        let mut new_path = self.path.clone();
        path.hash(&mut new_path);

        MapWithPath {
            path: new_path,
            map: self.map,
        }
    }
}

impl<'a, T> MapWithPathMut<'a, T> {
    #[inline]
    fn append_path(self, path: Uid) -> MapWithPathMut<'a, T> {
        use std::hash::Hash;

        let mut new_path = self.path.clone();
        path.hash(&mut new_path);

        MapWithPathMut {
            path: new_path,
            map: self.map,
        }
    }
}

impl<T> MapWithPathMut<'_, T> {
    #[inline]
    fn insert(&mut self, uid: Uid, val: T) -> Option<T> {
        use std::hash::{Hash, Hasher};

        let mut new_path = self.path.clone();
        uid.hash(&mut new_path);
        self.map.insert(Uid::new(new_path.finish() as u32), val)
    }
}

impl<'a, M> Index<Uid> for MapWithPathGen<M>
where
    M: Deref,
    M::Target: Index<Uid>,
{
    type Output = <M::Target as Index<Uid>>::Output;

    #[inline]
    fn index(&self, uid: Uid) -> &Self::Output {
        use std::hash::{Hash, Hasher};

        let mut new_path = self.path.clone();
        uid.hash(&mut new_path);
        &self.map[Uid::new(new_path.finish() as u32)]
    }
}

impl<'a, M> IndexMut<Uid> for MapWithPathGen<M>
where
    M: DerefMut,
    M::Target: IndexMut<Uid>,
{
    #[inline]
    fn index_mut(&mut self, uid: Uid) -> &mut Self::Output {
        use std::hash::{Hash, Hasher};

        let mut new_path = self.path.clone();
        uid.hash(&mut new_path);
        &mut self.map[Uid::new(new_path.finish() as u32)]
    }
}

impl<'a, M> Index<&'a Uid> for MapWithPathGen<M>
where
    M: Deref,
    M::Target: Index<Uid>,
{
    type Output = <M::Target as Index<Uid>>::Output;

    #[inline]
    fn index(&self, uid: &Uid) -> &Self::Output {
        use std::hash::{Hash, Hasher};

        let mut new_path = self.path.clone();
        uid.hash(&mut new_path);
        &self.map[Uid::new(new_path.finish() as u32)]
    }
}

impl<'a, M> IndexMut<&'a Uid> for MapWithPathGen<M>
where
    M: DerefMut,
    M::Target: IndexMut<Uid>,
{
    #[inline]
    fn index_mut(&mut self, uid: &Uid) -> &mut Self::Output {
        use std::hash::{Hash, Hasher};

        let mut new_path = self.path.clone();
        uid.hash(&mut new_path);
        &mut self.map[Uid::new(new_path.finish() as u32)]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FuncId(pub(crate) Uid);
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ComponentId(pub(crate) Uid);

impl fmt::Display for ComponentId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "%{}", (self.0))
    }
}

impl fmt::Display for FuncId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Func-{}", (self.0))
    }
}

// Weird and stupid trait to allow us to abstract over the fact that `Main` has a different
// signature compared to other functions (specifically, it has a static signature). I might
// make `Main` and other functions identical some day and remove this.
pub trait DefsAndFuncHelper {
    type FuncDef;

    fn get<'a, M: Index<Uid, Output = FuncDef<AnyInputSpec, AnyOutputSpec>> + ?Sized>(
        &'a self,
        map: &'a M,
    ) -> &'a Self::FuncDef;
}

pub trait DefsAndFuncHelperMut: DefsAndFuncHelper {
    type AsRef<'a>: DefsAndFuncHelper<FuncDef = Self::FuncDef> + Clone;

    fn as_ref(&self) -> Self::AsRef<'_>;

    fn get_mut<'a, M: IndexMut<Uid, Output = FuncDef<AnyInputSpec, AnyOutputSpec>> + ?Sized>(
        &'a mut self,
        map: &'a mut M,
    ) -> &'a mut Self::FuncDef;
}

impl DefsAndFuncHelper for FuncId {
    type FuncDef = FuncDef<AnyInputSpec, AnyOutputSpec>;

    #[inline]
    fn get<'a, M: Index<Uid, Output = FuncDef<AnyInputSpec, AnyOutputSpec>> + ?Sized>(
        &'a self,
        map: &'a M,
    ) -> &'a Self::FuncDef {
        &map[self.0]
    }
}

impl DefsAndFuncHelperMut for FuncId {
    type AsRef<'a> = Self;

    #[inline]
    fn as_ref(&self) -> Self::AsRef<'_> {
        *self
    }

    #[inline]
    fn get_mut<'a, M: IndexMut<Uid, Output = FuncDef<AnyInputSpec, AnyOutputSpec>> + ?Sized>(
        &'a mut self,
        map: &'a mut M,
    ) -> &'a mut Self::FuncDef {
        &mut map[self.0]
    }
}

impl<I, O, F> DefsAndFuncHelper for F
where
    F: Deref<Target = FuncDef<I, O>>,
    O: HasStorage<InternalWire>,
{
    type FuncDef = F::Target;

    #[inline]
    fn get<'a, M: Index<Uid, Output = FuncDef<AnyInputSpec, AnyOutputSpec>> + ?Sized>(
        &'a self,
        _: &'a M,
    ) -> &'a Self::FuncDef {
        self
    }
}

impl<I, O, F> DefsAndFuncHelperMut for F
where
    F: DerefMut<Target = FuncDef<I, O>>,
    O: HasStorage<InternalWire> + 'static,
    I: 'static,
{
    type AsRef<'any> = &'any FuncDef<I, O>;

    #[inline]
    fn as_ref(&self) -> Self::AsRef<'_> {
        &*self
    }

    #[inline]
    fn get_mut<'a, M: IndexMut<Uid, Output = FuncDef<AnyInputSpec, AnyOutputSpec>> + ?Sized>(
        &'a mut self,
        _: &'a mut M,
    ) -> &'a mut Self::FuncDef {
        self
    }
}

#[derive(Copy, Clone)]
pub struct DefsAndFunc<Map, Def> {
    def: Def,
    defs: Map,
}

impl<Map, Def> DefsAndFunc<Map, Def>
where
    Map: Deref,
    Map::Target: Index<Uid, Output = FuncDef<AnyInputSpec, AnyOutputSpec>>,
    Def: DefsAndFuncHelper,
{
    #[inline]
    pub fn def(&self) -> &Def::FuncDef {
        self.def.get(&*self.defs)
    }

    #[inline]
    pub fn get(&self, uid: FuncId) -> &FuncDef<AnyInputSpec, AnyOutputSpec> {
        &self.defs[uid.0]
    }
}

impl<Map, Def> DefsAndFunc<Map, Def>
where
    Map: DerefMut,
    Map::Target: IndexMut<Uid, Output = FuncDef<AnyInputSpec, AnyOutputSpec>>,
    Def: DefsAndFuncHelperMut,
{
    #[inline]
    pub fn def_mut(&mut self) -> &mut Def::FuncDef {
        self.def.get_mut(&mut *self.defs)
    }

    #[inline]
    pub fn get_mut(&mut self, uid: FuncId) -> &mut FuncDef<AnyInputSpec, AnyOutputSpec> {
        &mut self.defs[uid.0]
    }
}

pub struct Rack<C, InputSpec, OutputSpec>
where
    C: AnyComponent,
    OutputSpec: HasStorage<InternalWire>,
{
    uid_gen: UidGen,
    main: FuncDef<InputSpec, OutputSpec>,
    pub(crate) funcs: Funcs,
    pub(crate) meta_storage: UidMap<Meta<C>>,
    pub(crate) state_storage: UidMap<C>,
}

impl<C, InputSpec, OutputSpec> Rack<C, InputSpec, OutputSpec>
where
    C: AnyComponent,
    OutputSpec: HasStorage<InternalWire>,
    OutputSpec::Storage: Default,
{
    #[inline]
    pub fn new() -> Self {
        Rack {
            uid_gen: UidGen::new(),
            main: FuncDef::new(),
            funcs: Default::default(),
            meta_storage: Default::default(),
            state_storage: Default::default(),
        }
    }
}

impl<C, InputSpec, OutputSpec> Rack<C, InputSpec, OutputSpec>
where
    C: AnyComponent,
    OutputSpec: HasStorage<InternalWire>,
{
    #[inline]
    pub fn main(&self) -> FuncInstanceRef<'_, C, &FuncDef<InputSpec, OutputSpec>> {
        FuncInstanceRef {
            uid_gen: (),
            defs_and_func: DefsAndFunc {
                defs: &self.funcs,
                def: &self.main,
            },
            meta_storage: &self.meta_storage,
            state_storage: MapWithPath::new(&self.state_storage),
        }
    }

    #[inline]
    pub fn main_mut(&mut self) -> FuncInstanceMut<'_, C, &mut FuncDef<InputSpec, OutputSpec>> {
        FuncInstanceMut {
            uid_gen: &mut self.uid_gen,
            defs_and_func: DefsAndFunc {
                defs: &mut self.funcs,
                def: &mut self.main,
            },
            meta_storage: &mut self.meta_storage,
            state_storage: MapWithPathMut::new(&mut self.state_storage),
        }
    }

    #[inline]
    pub fn new_func(&mut self) -> FuncId {
        let id = self.uid_gen.next();
        self.funcs.insert(id, FuncDef::new());
        FuncId(id)
    }

    #[inline]
    pub fn func(&self, id: FuncId) -> FuncInstanceRef<'_, C, FuncId> {
        FuncInstanceRef {
            uid_gen: (),
            defs_and_func: DefsAndFunc {
                defs: &self.funcs,
                def: id,
            },
            meta_storage: &self.meta_storage,
            state_storage: MapWithPath::new(&self.state_storage),
        }
    }

    #[inline]
    pub fn func_mut(&mut self, id: FuncId) -> FuncInstanceMut<'_, C, FuncId> {
        FuncInstanceMut {
            uid_gen: &mut self.uid_gen,
            defs_and_func: DefsAndFunc {
                defs: &mut self.funcs,
                def: id,
            },
            meta_storage: &mut self.meta_storage,
            state_storage: MapWithPathMut::new(&mut self.state_storage),
        }
    }
}

impl<C, InputSpec, OutputSpec> Rack<C, InputSpec, OutputSpec>
where
    C: AnyComponent,
    OutputSpec: HasStorage<InternalWire>,
    InputSpec: RuntimeSpecifier + 'static,
    OutputSpec: RuntimeSpecifier + 'static,
{
    #[inline]
    pub fn update<Ctx>(&mut self, ctx: &Ctx)
    where
        Ctx: GetFunctionParam<InputSpec = InputSpec> + ContextMeta,
    {
        self.main_mut().update(ctx)
    }

    /// Get a specific output of this rack.
    /// #[inline]
    pub fn output<'a, Ctx: 'a>(
        &'a self,
        spec: OutputSpec,
        ctx: &'a Ctx,
    ) -> Option<PossiblyEither<C::OutputIter, Ctx::Iter>>
    where
        Ctx: GetFunctionParam<InputSpec = InputSpec> + ContextMeta,
    {
        self.main().output(spec, ctx)
    }
}

type Funcs = UidMap<FuncDef<AnyInputSpec, AnyOutputSpec>>;

pub struct FuncDef<InputSpec, OutputSpec>
where
    OutputSpec: HasStorage<InternalWire>,
{
    pub(crate) statements: Vec<ComponentId>,
    pub(crate) out_wires: OutputSpec::Storage,
    _marker: PhantomData<(InputSpec, OutputSpec)>,
}

#[derive(Clone, Debug)]
pub struct FuncInstanceGen<U, D, MS, SS> {
    uid_gen: U,
    pub(crate) defs_and_func: D,
    pub(crate) meta_storage: MS,
    pub(crate) state_storage: SS,
}

impl<U, M, D, MS, SS> FuncInstanceGen<U, DefsAndFunc<M, D>, MS, SS>
where
    M: Deref,
    M::Target: Index<Uid, Output = FuncDef<AnyInputSpec, AnyOutputSpec>>,
    D: DefsAndFuncHelper,
{
    #[inline]
    pub fn def(&self) -> &D::FuncDef {
        self.defs_and_func.def()
    }
}

impl<U, M, D, MS, SS> FuncInstanceGen<U, DefsAndFunc<M, D>, MS, SS>
where
    M: DerefMut,
    M::Target: IndexMut<Uid, Output = FuncDef<AnyInputSpec, AnyOutputSpec>>,
    D: DefsAndFuncHelperMut,
{
    #[inline]
    pub fn def_mut(&mut self) -> &mut D::FuncDef {
        self.defs_and_func.def_mut()
    }
}

pub type FuncInstanceRef<'a, C, Def> =
    FuncInstanceGen<(), DefsAndFunc<&'a Funcs, Def>, &'a UidMap<Meta<C>>, MapWithPath<'a, C>>;

pub type FuncInstanceMut<'a, C, Def> = FuncInstanceGen<
    &'a mut UidGen,
    DefsAndFunc<&'a mut Funcs, Def>,
    &'a mut UidMap<Meta<C>>,
    MapWithPathMut<'a, C>,
>;

impl<InputSpec, OutputSpec> FuncDef<InputSpec, OutputSpec>
where
    OutputSpec: HasStorage<InternalWire>,
    OutputSpec::Storage: Default,
{
    #[inline]
    fn new() -> Self {
        FuncDef {
            statements: Default::default(),
            out_wires: Default::default(),
            _marker: PhantomData,
        }
    }
}

pub trait FuncContext: ContextMeta {
    type MainCtx: GetFunctionParam + ContextMeta;
    type Component: AnyComponent;

    fn read_wire(
        &self,
        functions: &Funcs,
        wire: Wire<marker::Output>,
    ) -> Option<
        PossiblyEither<
            <Self::Component as AnyComponent>::OutputIter,
            <Self::MainCtx as GetFunctionParam>::Iter,
        >,
    >;
    fn state(&self) -> MapWithPath<'_, Self::Component>;
    fn meta(&self) -> &UidMap<Meta<Self::Component>>;
}

trait FuncContextMut: FuncContext {
    fn state_mut(&mut self) -> MapWithPathMut<'_, Self::Component>;
}

trait Update: FuncContextMut {
    fn update(&mut self, functions: &Funcs, statements: &[ComponentId]);
}

impl<T> Update for T
where
    T: FuncContextMut,
{
    #[inline]
    fn update(&mut self, functions: &Funcs, statements: &[ComponentId]) {
        for id in statements {
            match &self.meta()[id.0] {
                Meta::Component(cur_meta) => {
                    let new = self.state()[&id.0].update(&SingleComponentCtx {
                        ctx: &*self,
                        functions,
                        cur_meta,
                    });

                    self.state_mut().insert(id.0, new);
                }
                Meta::Function { func_id, .. } => {
                    let fid = func_id.0;

                    RecurseContext {
                        // We need to use `dyn` here because otherwise this type is infinitely recursive
                        // (Funnily, Rust doesn't notice this, it just hangs at the very last stage of
                        // compilation)
                        inner: self as &mut dyn FuncContextMut<
                            MainCtx = Self::MainCtx,
                            Component = Self::Component,
                        >,
                        path: *id,
                    }
                    .update(functions, &functions[fid].statements)
                }
            }
        }
    }
}

struct TopLevelContext<'a, Ctx, Component, State>
where
    Component: AnyComponent,
{
    ctx: &'a Ctx,
    state: State,
    meta: &'a UidMap<Meta<Component>>,
}

struct RecurseContext<Inner> {
    inner: Inner,
    path: ComponentId,
}

impl<Ctx, C, M> ContextMeta for TopLevelContext<'_, Ctx, C, M>
where
    C: AnyComponent,
    Ctx: ContextMeta,
{
    fn sample_rate(&self) -> u32 {
        self.ctx.sample_rate()
    }
}

impl<Ctx, Component, M> FuncContext for TopLevelContext<'_, Ctx, Component, MapWithPathGen<M>>
where
    Ctx: GetFunctionParam + ContextMeta,
    Ctx::InputSpec: RuntimeSpecifier,
    M: Deref<Target = UidMap<Component>>,
    Component: AnyComponent,
{
    type MainCtx = Ctx;
    type Component = Component;

    fn read_wire(
        &self,
        functions: &Funcs,
        Wire(wire): Wire<marker::Output>,
    ) -> Option<
        PossiblyEither<
            <Self::Component as AnyComponent>::OutputIter,
            <Self::MainCtx as GetFunctionParam>::Iter,
        >,
    > {
        match wire.element() {
            ElementSpecifier::Component { id } => {
                match &self.meta()[&id.0] {
                    Meta::Component(cur_meta) => {
                        let comp = &self.state()[&id.0];

                        Some(PossiblyEither::Left(comp.output(
                            AnyOutputSpec(wire.output_id().0),
                            &SingleComponentCtx {
                                ctx: &*self,
                                functions,
                                cur_meta,
                            },
                        )))
                    }
                    Meta::Function { func_id, .. } => {
                        RecurseContext {
                            // We need to use `dyn` here because otherwise this type is infinitely recursive
                            // (Funnily, Rust doesn't notice this, it just hangs at the very last stage of
                            // compilation)
                            inner: self as &dyn FuncContext<
                                MainCtx = Self::MainCtx,
                                Component = Self::Component,
                            >,
                            path: id,
                        }
                        .read_wire(
                            functions,
                            functions[func_id.0]
                                .out_wires
                                .get(&wire.output_id())
                                .as_ref()?
                                .clone(),
                        )
                    }
                }
            }
            ElementSpecifier::FuncInputs => self
                .ctx
                .input(Ctx::InputSpec::from_id(wire.io_index))
                .map(PossiblyEither::Right),
        }
    }

    fn state(&self) -> MapWithPath<'_, Self::Component> {
        self.state.as_ref()
    }

    fn meta(&self) -> &UidMap<Meta<Self::Component>> {
        self.meta
    }
}

impl<Ctx, Component, M> FuncContextMut for TopLevelContext<'_, Ctx, Component, MapWithPathGen<M>>
where
    Ctx: GetFunctionParam + ContextMeta,
    Ctx::InputSpec: RuntimeSpecifier,
    M: DerefMut<Target = UidMap<Component>>,
    Component: AnyComponent,
{
    fn state_mut(&mut self) -> MapWithPathMut<'_, Self::Component> {
        self.state.as_mut()
    }
}

impl<Inner> ContextMeta for RecurseContext<Inner>
where
    Inner: Deref,
    Inner::Target: ContextMeta,
{
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
}

impl<Inner> FuncContext for RecurseContext<Inner>
where
    Inner: Deref,
    Inner::Target: FuncContext,
{
    type MainCtx = <Inner::Target as FuncContext>::MainCtx;
    type Component = <Inner::Target as FuncContext>::Component;

    fn read_wire(
        &self,
        functions: &Funcs,
        Wire(wire): Wire<marker::Output>,
    ) -> Option<
        PossiblyEither<
            <Self::Component as AnyComponent>::OutputIter,
            <Self::MainCtx as GetFunctionParam>::Iter,
        >,
    > {
        match wire.element() {
            ElementSpecifier::Component { id } => {
                let cur_meta = self.meta()[&id.0].component().unwrap();
                let comp = &self.state()[&id.0];

                let out = comp.output(
                    AnyOutputSpec(wire.output_id().0),
                    &SingleComponentCtx {
                        ctx: &*self,
                        functions,
                        cur_meta,
                    },
                );

                Some(PossiblyEither::Left(out))
            }
            ElementSpecifier::FuncInputs => match &self.inner.meta()[self.path.0] {
                Meta::Function { inputs, .. } => {
                    match inputs.get(&AnyInputSpec(wire.output_id().0)) {
                        Some(wire) => self.inner.read_wire(functions, *wire),
                        None => None,
                    }
                }
                _ => unreachable!(),
            },
        }
    }

    fn state(&self) -> MapWithPath<'_, Self::Component> {
        self.inner.state().append_path(self.path.0)
    }

    fn meta(&self) -> &UidMap<Meta<Self::Component>> {
        self.inner.meta()
    }
}

impl<Inner> FuncContextMut for RecurseContext<Inner>
where
    Inner: DerefMut,
    Inner::Target: FuncContextMut,
{
    fn state_mut(&mut self) -> MapWithPathMut<'_, Self::Component> {
        self.inner.state_mut().append_path(self.path.0)
    }
}

impl<C, InputSpec, OutputSpec, Def, M>
    FuncInstanceGen<
        &'_ mut UidGen,
        DefsAndFunc<M, Def>,
        &'_ mut UidMap<Meta<C>>,
        MapWithPathMut<'_, C>,
    >
where
    InputSpec: RuntimeSpecifier,
    OutputSpec: RuntimeSpecifier + HasStorage<InternalWire>,
    C: AnyComponent,
    M: Deref<Target = Funcs>,
    Def: DefsAndFuncHelper<FuncDef = FuncDef<InputSpec, OutputSpec>> + Clone,
{
}

impl<C, InputSpec, OutputSpec, Def> FuncInstanceMut<'_, C, Def>
where
    InputSpec: RuntimeSpecifier,
    OutputSpec: RuntimeSpecifier + HasStorage<InternalWire>,
    C: AnyComponent,
    Def: DefsAndFuncHelperMut<FuncDef = FuncDef<InputSpec, OutputSpec>>,
{
    // TODO: Return a result
    // #[inline]
    pub fn wire(&mut self, src: WireSrc, dst: WireDst) {
        match dst.0 {
            WireDstInner::Input(dst) => match dst.element() {
                ElementSpecifier::Component { id } => {
                    *self.meta_storage[&id.0]
                        .inputs_mut()
                        .get_mut(&dst.input_id()) = Some(src);
                }
                ElementSpecifier::FuncInputs => {
                    *self
                        .def_mut()
                        .out_wires
                        .get_mut(&OutputSpec::from_id(dst.input_id().0)) = Some(src)
                }
            },
            WireDstInner::Param(val, dst) => match dst.element() {
                ElementSpecifier::Component { id } => {
                    // TODO: Allow functions to have parameters
                    *self.meta_storage[&id.0]
                        .component_mut()
                        .unwrap()
                        .params
                        .get_mut(&dst.param_id())
                        .1
                        .downcast_mut::<InternalParamWire>()
                        .unwrap() = Some(ParamWire { value: val, src })
                }
                ElementSpecifier::FuncInputs => unimplemented!(),
            },
        }
    }

    #[inline]
    pub fn update<Ctx>(&mut self, ctx: &Ctx)
    where
        Ctx: GetFunctionParam<InputSpec = InputSpec> + ContextMeta,
    {
        TopLevelContext {
            ctx,
            meta: &mut *self.meta_storage,
            state: self.state_storage.as_mut(),
        }
        .update(
            &self.defs_and_func.defs,
            &self.defs_and_func.def().statements,
        )
    }

    #[inline]
    pub fn set_param<S: RuntimeSpecifier, V: 'static>(
        &mut self,
        component: ComponentId,
        param: S,
        value: V,
    ) {
        let (v, _) = self.meta_storage[&component.0]
            .component_mut()
            .unwrap()
            .params
            .get_mut(&AnyParamSpec(param.id()));
        let val = v.downcast_mut::<V>().expect("Incorrect param type");
        *val = value;
    }

    #[inline]
    pub fn push_component(&mut self, component: impl Into<C>) -> ComponentId {
        let component = component.into();
        let params = component.param_default();
        let inputs = component.input_default();

        let uid = self.uid_gen.next();

        self.meta_storage
            .insert(uid, Meta::Component(ComponentMeta { inputs, params }));
        self.state_storage.insert(uid, component);
        let cid = ComponentId(uid);
        self.def_mut().statements.push(cid);

        cid
    }
}

impl<C, InputSpec, OutputSpec, Def> FuncInstanceMut<'_, C, Def>
where
    InputSpec: RuntimeSpecifier,
    OutputSpec: RuntimeSpecifier + HasStorage<InternalWire>,
    C: AnyComponent + Clone,
    Def: DefsAndFuncHelperMut<FuncDef = FuncDef<InputSpec, OutputSpec>>,
{
    #[inline]
    pub fn push_function_call(&mut self, fid: FuncId) -> ComponentId {
        fn add_function_state<C>(
            defs: &Funcs,
            meta: &UidMap<Meta<C>>,
            mut state: MapWithPathMut<'_, C>,
            statements: &[ComponentId],
        ) where
            C: AnyComponent + Clone,
        {
            for id in statements {
                match &meta[id.0] {
                    Meta::Component { .. } => {
                        state.insert(id.0, state.original_map()[&id.0].clone());
                    }
                    Meta::Function { func_id, .. } => add_function_state(
                        defs,
                        meta,
                        state.as_mut().append_path(id.0),
                        &defs[func_id.0].statements,
                    ),
                }
            }
        }

        let new_id = self.uid_gen.next();
        self.meta_storage.insert(
            new_id,
            Meta::Function {
                func_id: fid,
                inputs: Default::default(),
            },
        );
        self.def_mut().statements.push(ComponentId(new_id));

        add_function_state(
            &*self.defs_and_func.defs,
            self.meta_storage,
            self.state_storage.as_mut().append_path(new_id),
            &self.defs_and_func.get(fid).statements,
        );

        ComponentId(new_id)
    }
}

impl<C, InputSpec, OutputSpec, Def> FuncInstanceRef<'_, C, Def>
where
    InputSpec: RuntimeSpecifier,
    OutputSpec: RuntimeSpecifier + HasStorage<InternalWire>,
    Def: DefsAndFuncHelper<FuncDef = FuncDef<InputSpec, OutputSpec>> + Clone,
    C: AnyComponent,
{
    /// Get a specific output of this function.
    /// #[inline]
    pub fn output<'a, Ctx: 'a>(
        &'a self,
        spec: OutputSpec,
        ctx: &'a Ctx,
    ) -> Option<PossiblyEither<C::OutputIter, Ctx::Iter>>
    where
        Ctx: GetFunctionParam<InputSpec = InputSpec> + ContextMeta,
    {
        let wire = (*self.def().out_wires.get(&spec))?;

        TopLevelContext {
            ctx,
            state: self.state_storage.as_ref(),
            meta: self.meta_storage,
        }
        .read_wire(self.defs_and_func.defs, wire)
    }
}

pub struct SingleComponentCtx<'a, Ctx, C>
where
    C: AnyComponent,
{
    ctx: &'a Ctx,
    functions: &'a Funcs,
    cur_meta: &'a ComponentMeta<C>,
}

impl<'a, Ctx, C> ContextMeta for SingleComponentCtx<'a, Ctx, C>
where
    C: AnyComponent,
    Ctx: ContextMeta,
{
    #[inline]
    fn sample_rate(&self) -> u32 {
        self.ctx.sample_rate()
    }
}

impl<'a, Ctx, C> AnyMeta for SingleComponentCtx<'a, Ctx, C>
where
    C: AnyComponent,
{
    type ParamStorage = C::ParamStorage;
    type InputStorage = C::InputStorage;

    #[inline]
    fn params(&self) -> &Self::ParamStorage {
        &self.cur_meta.params
    }

    #[inline]
    fn inputs(&self) -> &Self::InputStorage {
        &self.cur_meta.inputs
    }
}

impl<'a, Ctx, C> AnyContext for SingleComponentCtx<'a, Ctx, C>
where
    C: AnyComponent,
    Ctx: FuncContext<Component = C>,
{
    type Iter = PossiblyEither<C::OutputIter, <Ctx::MainCtx as GetFunctionParam>::Iter>;

    #[inline]
    fn read_wire(&self, wire: WireSrc) -> Option<Self::Iter> {
        self.ctx.read_wire(self.functions, wire)
    }
}
