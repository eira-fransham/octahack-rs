use crate::{
    components::{
        anycomponent::{AnyMeta, AnyUiElement, AnyUiElementDisplayParamValue},
        EnumerateValues,
    },
    params::{HasStorage, ParamStorage, Storage},
    rack::{
        DefsAndFuncHelper, ElementSpecifier, FuncDef, FuncId, FuncInstanceRef, GenericWire,
        InternalParamWire, InternalWire, MapWithPath, Meta, Wire, WireSrc,
    },
    AnyComponent, AnyInputSpec, AnyOutputSpec, AnyParamSpec, Rack, RefRuntimeSpecifier,
    RuntimeSpecifier, SpecId, Uid, UidGen, UidMap, Value, XOrHasher,
};
use std::{fmt, marker::PhantomData};

struct FnDisplay<F: Fn(&mut fmt::Formatter) -> fmt::Result>(F);

impl<F> fmt::Display for FnDisplay<F>
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self.0)(f)
    }
}

#[inline]
fn print_spec<ISpec: RefRuntimeSpecifier + ?Sized>(i: &ISpec) -> impl fmt::Display + '_ {
    FnDisplay(move |f| write!(f, "{}: {}", i, i.value_type()))
}

struct DisplayFunc<'a, C, Def, I, O, N>
where
    C: AnyComponent,
{
    func: FuncInstanceRef<'a, C, Def>,
    inputs: I,
    outputs: O,
    name: N,
}

impl<C, InputSpec, OutputSpec, Def, I, O, N> fmt::Display for DisplayFunc<'_, C, Def, I, O, N>
where
    Def: DefsAndFuncHelper<FuncDef = FuncDef<InputSpec, OutputSpec>>,
    I: fmt::Display,
    O: fmt::Display,
    N: fmt::Display,
    InputSpec: RuntimeSpecifier,
    OutputSpec: fmt::Display + HasStorage<InternalWire>,
    for<'any> &'any OutputSpec::Storage: IntoIterator<Item = (OutputSpec, &'any InternalWire)>,
    C: AnyComponent,
    for<'any> &'any C:
        AnyUiElement<'any> + AnyUiElementDisplayParamValue<'any, ParamStorage = C::ParamStorage>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (print_wire, print_opt_wire) = {
            #[inline]
            fn print_wire_inner<'a, ISpec, C>(
                wire: &'a WireSrc,
                meta: &'a UidMap<Meta<C>>,
                components: MapWithPath<'a, C>,
            ) -> impl fmt::Display + 'a
            where
                ISpec: RuntimeSpecifier,
                C: AnyComponent,
                for<'any> &'any C: AnyUiElement<'any>,
            {
                FnDisplay(move |f| match wire {
                    Wire(GenericWire {
                        io_index,
                        element: ElementSpecifier::FuncInputs,
                        ..
                    }) => write!(f, "{}", ISpec::from_id(*io_index)),
                    Wire(GenericWire {
                        io_index,
                        element: ElementSpecifier::Component { id },
                        ..
                    }) => match &meta[id.0] {
                        Meta::Component(_) => write!(
                            f,
                            "{}->{}",
                            id,
                            components[&id.0].output_names().nth(*io_index).unwrap()
                        ),

                        Meta::Function { .. } => write!(f, "{}->{}", id, AnyOutputSpec(*io_index)),
                    },
                })
            }

            #[inline]
            fn print_opt_wire_inner<'a, ISpec, C>(
                wire: &'a InternalWire,
                meta: &'a UidMap<Meta<C>>,
                components: MapWithPath<'a, C>,
            ) -> impl fmt::Display + 'a
            where
                ISpec: RuntimeSpecifier,
                C: AnyComponent,
                for<'any> &'any C: AnyUiElement<'any>,
            {
                FnDisplay(move |f| match wire {
                    None => write!(f, "NONE"),
                    Some(wire) => write!(
                        f,
                        "{}",
                        print_wire_inner::<ISpec, _>(wire, meta, components.as_ref())
                    ),
                })
            }

            (
                |wire| {
                    print_wire_inner::<InputSpec, _>(
                        wire,
                        &self.func.meta_storage,
                        self.func.state_storage.as_ref(),
                    )
                },
                |wire| {
                    print_opt_wire_inner::<InputSpec, _>(
                        wire,
                        &self.func.meta_storage,
                        self.func.state_storage.as_ref(),
                    )
                },
            )
        };

        writeln!(f, "def {} {} -> {}:", self.name, self.inputs, self.outputs)?;

        for i in &self.func.def().statements {
            match &self.func.meta_storage[i.0] {
                Meta::Component(meta) => {
                    write!(f, "    {}: {{", i)?;
                    let component = &self.func.state_storage[&i.0];
                    let mut onameiter = component.output_names();

                    if let Some(o) = onameiter.next() {
                        write!(f, " {}", print_spec(o))?;
                    }

                    for o in onameiter {
                        write!(f, ", {}", print_spec(o))?;
                    }

                    writeln!(f, " }} = {} {{", component.name())?;

                    for (i, p) in component.param_names().enumerate() {
                        write!(f, "        ${} = ", p)?;

                        let (value, param_wire) = meta.params.get(&AnyParamSpec(i));

                        match param_wire.downcast_ref::<InternalParamWire>().unwrap() {
                            Some(wire) => {
                                write!(
                                    f,
                                    "lerp({}, {}..{})",
                                    print_wire(&wire.src),
                                    component.display_param_value(AnyParamSpec(i), value),
                                    component.display_param_value(AnyParamSpec(i), &wire.value),
                                )?;
                            }
                            None => {
                                write!(
                                    f,
                                    "{}",
                                    component.display_param_value(AnyParamSpec(i), value)
                                )?;
                            }
                        }

                        writeln!(f, ",")?;
                    }

                    let inameiter = component.input_names().enumerate();

                    if !inameiter.is_empty() {
                        writeln!(f)?;
                    }

                    for (i, input) in inameiter {
                        let wire = meta.inputs.get(&AnyInputSpec(i));

                        writeln!(f, "        {} = {},", input, print_opt_wire(wire))?;
                    }

                    writeln!(f, "    }}")?;
                }
                Meta::Function { func_id, inputs } => {
                    writeln!(f, "    {} = {} {{ ", i, func_id)?;

                    for (name, wire) in inputs {
                        writeln!(f, "        {} = {}", name, print_opt_wire(wire))?;
                    }

                    writeln!(f, "    }}")?;
                }
            }
        }

        writeln!(f)?;
        writeln!(f, "    return {{")?;

        for (o, wire) in &self.func.def().out_wires {
            writeln!(f, "        {} = {},", o, print_opt_wire(wire))?;
        }

        writeln!(f, "    }}")?;

        Ok(())
    }
}

impl<C, InputSpec, OutputSpec> fmt::Display for Rack<C, InputSpec, OutputSpec>
where
    C: AnyComponent,
    InputSpec: EnumerateValues,
    OutputSpec: EnumerateValues + HasStorage<InternalWire>,
    for<'any> &'any OutputSpec::Storage: IntoIterator<Item = (OutputSpec, &'any InternalWire)>,
    for<'any> &'any C:
        AnyUiElement<'any> + AnyUiElementDisplayParamValue<'any, ParamStorage = C::ParamStorage>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct DisplaySpec<S>(PhantomData<S>);

        impl<S> fmt::Display for DisplaySpec<S>
        where
            S: EnumerateValues,
        {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{{ ")?;
                let mut ispeciter = S::values();

                if let Some(i) = ispeciter.next() {
                    write!(f, "{}", print_spec(i))?;
                }

                for i in ispeciter {
                    write!(f, ", {}", print_spec(i))?;
                }

                write!(f, " }}")
            }
        }

        struct DisplayAnySpec<S>(S);

        impl<S> fmt::Display for DisplayAnySpec<S>
        where
            S: Iterator + Clone,
            S::Item: RefRuntimeSpecifier,
        {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{{ ")?;
                let mut ispeciter = self.0.clone();

                if let Some(i) = ispeciter.next() {
                    write!(f, "{}", print_spec(&i))?;
                }

                for i in ispeciter {
                    write!(f, ", {}", print_spec(&i))?;
                }

                write!(f, " }}")
            }
        }

        for (id, func) in &self.funcs {
            let id = FuncId(id);

            writeln!(
                f,
                "{}",
                DisplayFunc {
                    func: self.func(id),
                    inputs: "{ .. }",
                    outputs: DisplayAnySpec(func.out_wires.into_iter().map(|(s, _)| s)),
                    name: id,
                }
            )?;
        }

        writeln!(
            f,
            "{}",
            DisplayFunc {
                func: self.main(),
                inputs: DisplaySpec::<InputSpec>(PhantomData),
                outputs: DisplaySpec::<OutputSpec>(PhantomData),
                name: "Main"
            }
        )
    }
}
