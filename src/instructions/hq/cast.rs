use super::super::prelude::*;

#[derive(Clone, Debug)]
pub struct Fields(pub IrType);

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "to": "{}"
    }}"#,
            self.0
        )
    }
}

fn best_cast_candidate(from: IrType, to: IrType) -> HQResult<IrType> {
    let to_base_types = to.base_types().collect::<Vec<_>>();
    hq_assert!(!to_base_types.is_empty());
    let Some(from_base) = from.base_type() else {
        hq_bug!("from type has no base type")
    };
    Ok(if to_base_types.contains(&from_base) {
        from_base
    } else {
        let mut candidates = vec![];
        for preference in match from_base {
            IrType::Int => &[
                IrType::Float,
                IrType::String,
                IrType::ColorARGB,
                IrType::Boolean,
            ] as &[IrType],
            IrType::Boolean => &[
                IrType::Int,
                IrType::Float,
                IrType::String,
                IrType::ColorARGB,
            ] as &[IrType],
            IrType::Float => &[
                IrType::String,
                IrType::Int,
                IrType::Boolean,
                IrType::ColorARGB,
            ] as &[IrType],
            IrType::String => &[
                IrType::Float,
                IrType::Int,
                IrType::ColorRGB,
                IrType::Boolean,
            ] as &[IrType],
            IrType::ColorRGB | IrType::ColorARGB => hq_bad_proj!(
                "should not be casting from colour. this is probably a project using 'hacked' \
                 blocks, or a bug"
            ),
            _ => unreachable!(),
        } {
            if to_base_types.contains(preference) {
                candidates.push(preference);
            }
        }
        hq_assert!(!candidates.is_empty());
        *candidates[0]
    })
}

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    &Fields(to): &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let from = inputs[0];

    let target = best_cast_candidate(from, to)?;

    let Some(from_base) = from.base_type() else {
        hq_bug!("from type has no base type")
    };

    Ok(match target {
        IrType::Float => match from_base {
            IrType::Float => wasm![],
            IrType::Int | IrType::Boolean => wasm![F64ConvertI32S],
            IrType::String => {
                let func_index = func.registries().external_functions().register(
                    ("cast", "string2float".into()),
                    (vec![ValType::EXTERNREF], vec![ValType::F64]),
                )?;
                wasm![Call(func_index)]
            }
            _ => hq_todo!("bad cast: {:?} -> float", from_base),
        },
        IrType::String => match from_base {
            IrType::Float => {
                let func_index = func.registries().external_functions().register(
                    ("cast", "float2string".into()),
                    (vec![ValType::F64], vec![ValType::EXTERNREF]),
                )?;
                wasm![Call(func_index)]
            }
            IrType::Int => {
                let func_index = func.registries().external_functions().register(
                    ("cast", "int2string".into()),
                    (vec![ValType::I32], vec![ValType::EXTERNREF]),
                )?;
                wasm![Call(func_index)]
            }
            IrType::Boolean => {
                let true_string = func
                    .registries()
                    .strings()
                    .register_default("true".into())?;
                let false_string = func
                    .registries()
                    .strings()
                    .register_default("false".into())?;
                let bool_local = func.local(ValType::I32)?;
                wasm![
                    LocalSet(bool_local),
                    GlobalGet(true_string),
                    GlobalGet(false_string),
                    LocalGet(bool_local),
                    TypedSelect(ValType::EXTERNREF),
                ]
            }
            IrType::String => vec![],
            _ => hq_todo!("bad cast: {:?} -> string", from_base),
        },
        IrType::Int => match from_base {
            IrType::Float => wasm![I32TruncSatF64S],
            IrType::String => {
                let func_index = func.registries().external_functions().register(
                    ("cast", "string2float".into()),
                    (vec![ValType::EXTERNREF], vec![ValType::F64]),
                )?;
                wasm![Call(func_index), I32TruncSatF64S]
            }
            IrType::Int | IrType::Boolean => vec![],
            _ => hq_todo!("unimplemented cast: {:?} -> Int", from_base),
        },
        IrType::Boolean => match from_base {
            IrType::Float => wasm![I32TruncSatF64S],
            IrType::String => {
                let strict_eq = func.registries().external_functions().register(
                    ("wasm:js-string", "equals".into()),
                    (
                        vec![ValType::EXTERNREF, ValType::EXTERNREF],
                        vec![ValType::I32],
                    ),
                )?;
                let loose_eq = func.registries().external_functions().register(
                    ("operator", "eq_string".into()),
                    (
                        vec![ValType::EXTERNREF, ValType::EXTERNREF],
                        vec![ValType::I32],
                    ),
                )?;
                let string_local = func.local(ValType::EXTERNREF)?;
                let empty_string = func.registries().strings().register_default("".into())?;
                let zero_string = func.registries().strings().register_default("0".into())?;
                let false_string = func
                    .registries()
                    .strings()
                    .register_default("false".into())?;
                wasm![
                    LocalTee(string_local),
                    GlobalGet(empty_string),
                    Call(strict_eq),
                    LocalGet(string_local),
                    GlobalGet(zero_string),
                    Call(strict_eq),
                    LocalGet(string_local),
                    GlobalGet(false_string),
                    Call(loose_eq),
                    I32Or,
                    I32Or,
                ]
            }
            IrType::Int => wasm![I32Eqz, I32Eqz],
            IrType::Boolean => vec![],
            _ => hq_todo!("unimplemented cast: {:?} -> Int", from_base),
        },
        IrType::ColorARGB => match from_base {
            IrType::Float => wasm![I32TruncSatF64S],
            IrType::ColorRGB | IrType::ColorARGB | IrType::Int | IrType::Boolean => wasm![],
            _ => hq_todo!("unimplemented cast: {:?} -> ColorARGB", from_base),
        },
        IrType::ColorRGB => match from_base {
            IrType::ColorRGB | IrType::ColorARGB | IrType::Int | IrType::Boolean => wasm![],
            IrType::String => {
                let func_index = func.registries().external_functions().register(
                    ("cast", "string2colorrgb".into()),
                    (vec![ValType::EXTERNREF], vec![ValType::I32]),
                )?;
                wasm![Call(func_index)]
            }
            _ => hq_todo!("unimplemented cast: {:?} -> ColorRGB", from_base),
        },
        _ => hq_todo!("unimplemented cast: {:?} -> {:?}", from_base, target),
    })
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Number
        .or(IrType::String)
        .or(IrType::Boolean)]))
}

pub fn output_type(inputs: Rc<[IrType]>, &Fields(to): &Fields) -> HQResult<ReturnType> {
    Ok(Singleton(
        inputs[0]
            .base_types()
            .map(|from| Ok((from, best_cast_candidate(from, to)?)))
            .collect::<HQResult<Vec<_>>>()?
            .into_iter()
            .map(|(from, to_candidate)| {
                Ok(
                    match (
                        from.base_type()
                            .ok_or_else(|| make_hq_bug!("type has no base type"))?,
                        to_candidate,
                    ) {
                        (IrType::String, IrType::Float) => IrType::none_if_false(
                            from.contains(IrType::StringNumber),
                            IrType::FloatNotNan,
                        )
                        .or(IrType::none_if_false(
                            from.intersects(IrType::StringBoolean.or(IrType::StringNan)),
                            IrType::FloatNan,
                        )),
                        (IrType::String, IrType::Int) => IrType::Int,
                        (IrType::String, IrType::Boolean) => IrType::Boolean,
                        (IrType::Float, IrType::String) => IrType::none_if_false(
                            from.contains(IrType::FloatNan),
                            IrType::StringNan,
                        )
                        .or(IrType::none_if_false(
                            from.intersects(IrType::FloatNotNan),
                            IrType::StringNumber,
                        )),
                        (IrType::Float, IrType::Int) => {
                            IrType::none_if_false(from.maybe_negative(), IrType::IntNeg)
                                .or(IrType::none_if_false(from.maybe_positive(), IrType::IntPos))
                                .or(IrType::none_if_false(
                                    from.maybe_zero() || from.maybe_nan(),
                                    IrType::IntZero,
                                ))
                        }
                        (IrType::Float, IrType::Boolean) => IrType::none_if_false(
                            from.maybe_negative() || from.maybe_positive(),
                            IrType::BooleanTrue,
                        )
                        .or(IrType::none_if_false(
                            from.maybe_zero() || from.maybe_nan(),
                            IrType::BooleanFalse,
                        )),
                        (IrType::Int | IrType::Boolean, IrType::Float) => {
                            IrType::none_if_false(from.maybe_negative(), IrType::FloatNegInt)
                                .or(IrType::none_if_false(
                                    from.maybe_positive(),
                                    IrType::FloatPosInt,
                                ))
                                .or(IrType::none_if_false(
                                    from.maybe_zero(),
                                    IrType::FloatPosZero,
                                ))
                        }
                        (IrType::Int, IrType::Boolean) => IrType::none_if_false(
                            from.maybe_positive() | from.maybe_negative(),
                            IrType::BooleanFalse,
                        )
                        .or(IrType::none_if_false(
                            from.maybe_zero(),
                            IrType::BooleanFalse,
                        )),
                        (IrType::Boolean, IrType::Int) => {
                            IrType::none_if_false(from.maybe_positive(), IrType::IntPos)
                                .or(IrType::none_if_false(from.maybe_zero(), IrType::IntZero))
                        }
                        (IrType::Int, IrType::String) => IrType::StringNumber,
                        (IrType::Boolean, IrType::String) => IrType::StringBoolean,
                        (_, IrType::ColorRGB) => IrType::ColorRGB,
                        (_, IrType::ColorARGB) => IrType::ColorARGB,
                        (other_from, other_to) if other_to.contains(other_from) => other_from,
                        _ => hq_bug!("bad cast: {} -> {}", from, to),
                    },
                )
            })
            .collect::<HQResult<Vec<_>>>()?
            .into_iter()
            .reduce(IrType::or)
            .ok_or_else(|| make_hq_bug!("input type was empty"))?,
    ))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub fn const_fold(
    inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
    &Fields(to): &Fields,
) -> HQResult<ConstFold> {
    Ok(match &inputs[0] {
        ConstFoldItem::Unknown { .. } | ConstFoldItem::Boxed(_, _) => NotFoldable,
        ConstFoldItem::Basic(val) => {
            ConstFold::Folded(Rc::from([ConstFoldItem::Basic(match val {
                VarVal::String(string) => match best_cast_candidate(IrType::String, to)? {
                    IrType::Boolean => VarVal::Bool(
                        &**string != "0" && !string.is_empty() && string.to_lowercase() != "false",
                    ),
                    IrType::String => VarVal::String(string.clone()),
                    IrType::Float => VarVal::Float(string.parse().unwrap_or(f64::NAN)),

                    #[expect(clippy::cast_possible_truncation, reason = "this is deliberate")]
                    IrType::Int => VarVal::Int(string.parse::<f64>().unwrap_or(0.0) as i32),
                    _ => hq_bug!("tried to cast from string to {}", to),
                },
                VarVal::Float(float) => match best_cast_candidate(IrType::Float, to)? {
                    IrType::Boolean => VarVal::Bool(*float != 0.0),
                    IrType::Float => VarVal::Float(*float),
                    #[expect(clippy::cast_possible_truncation, reason = "deliberate truncation")]
                    IrType::Int => VarVal::Int(*float as i32),
                    IrType::String => VarVal::String(float.to_string().into_boxed_str()),
                    _ => hq_bug!("tried to cast from float to {}", to),
                },
                VarVal::Int(int) => match best_cast_candidate(IrType::Int, to)? {
                    IrType::Boolean => VarVal::Bool(*int != 0),
                    IrType::Float => VarVal::Float((*int).into()),
                    IrType::Int => VarVal::Int(*int),
                    IrType::String => VarVal::String(int.to_string().into_boxed_str()),
                    _ => hq_bug!("tried to cast from int to {}", to),
                },
                VarVal::Bool(b) => match best_cast_candidate(IrType::Boolean, to)? {
                    IrType::Boolean => VarVal::Bool(*b),
                    IrType::Float => VarVal::Float(if *b { 1.0 } else { 0.0 }),
                    IrType::Int => VarVal::Int((*b).into()),
                    IrType::String => VarVal::String(b.to_string().into_boxed_str()),
                    _ => hq_bug!("tried to cast from bool to {}", to),
                },
            })]))
        }
        ConstFoldItem::Stack(_) => hq_bug!("found ConstFoldItem::Stack on stack"),
    })
}

crate::instructions_test! {float; hq_cast; t @ super::Fields(IrType::Float)}
crate::instructions_test! {string; hq_cast; t @ super::Fields(IrType::String)}
crate::instructions_test! {int; hq_cast; t @ super::Fields(IrType::Int)}
crate::instructions_test! {boolean; hq_cast; t @ super::Fields(IrType::Boolean)}
