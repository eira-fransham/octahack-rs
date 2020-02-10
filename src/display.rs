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
    FnDisplay(move |f| write!(f, "{}", i))
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

                    write!(f, " }} = {}", component.name())?;

                    let (params, inputs) = (
                        component.param_names().enumerate(),
                        component.input_names().enumerate(),
                    );

                    if params.len() + inputs.len() > 0 {
                        writeln!(f, " {{")?;
                        for (i, p) in params {
                            write!(f, "        ${} = ", p)?;

                            let (value, param_wire) = meta.params.get(&AnyParamSpec(i));

                            write!(
                                f,
                                "{}",
                                component.display_param_value(AnyParamSpec(i), value)
                            )?;
                            if let Some(wire) =
                                param_wire.downcast_ref::<InternalParamWire>().unwrap()
                            {
                                write!(
                                    f,
                                    " + {} * {}",
                                    print_wire(&wire.src),
                                    &wire.cv.natural_value
                                )?;
                                assert!(wire.cv.wire.is_none());
                            }

                            writeln!(f, ",")?;
                        }

                        if inputs.len() > 0 {
                            writeln!(f)?;
                        }

                        for (i, input) in inputs {
                            let wire = meta.inputs.get(&AnyInputSpec(i));

                            writeln!(f, "        {} = {},", input, print_opt_wire(wire))?;
                        }

                        writeln!(f, "    }}")?;
                    }
                }
                Meta::Function { func_id, inputs } => {
                    write!(f, "    {}: {{ ", i)?;

                    let mut iter = (&self.func.defs_and_func.get(*func_id).out_wires).into_iter();
                    if let Some((o, _)) = iter.next() {
                        write!(f, "{}", print_spec(&o))?;
                    }
                    for (o, _) in iter {
                        write!(f, ", {}", print_spec(&o))?;
                    }
                    write!(f, " }} = {}", func_id)?;

                    let mut any = false;

                    for (name, wire) in inputs {
                        if !any {
                            writeln!(f, " {{")?;
                        }

                        writeln!(f, "        {} = {}", name, print_opt_wire(wire))?;
                        any = true;
                    }

                    if any {
                        writeln!(f, "    }}")?;
                    } else {
                        writeln!(f)?;
                    }
                }
            }
        }

        writeln!(f)?;
        writeln!(f, "    return {{")?;

        for (o, wire) in &self.func.def().out_wires {
            writeln!(f, "        {} = {},", o, print_opt_wire(wire))?;
        }

        write!(f, "    }}")?;

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
            writeln!(f)?;
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
