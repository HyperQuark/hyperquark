use super::super::prelude::*;

#[derive(Clone, Debug)]
pub struct Fields(pub IrType);

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
            IrType::QuasiInt => &[IrType::Float, IrType::String] as &[IrType],
            IrType::Float => &[IrType::String, IrType::QuasiInt] as &[IrType],
            IrType::String => &[IrType::Float, IrType::QuasiInt] as &[IrType],
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
            IrType::QuasiInt => wasm![F64ConvertI32S],
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
            IrType::QuasiInt => {
                let func_index = func.registries().external_functions().register(
                    ("cast", "int2string".into()),
                    (vec![ValType::I32], vec![ValType::EXTERNREF]),
                )?;
                wasm![Call(func_index)]
            }
            IrType::String => vec![],
            _ => hq_todo!("bad cast: {:?} -> string", from_base),
        },
        IrType::QuasiInt => match from_base {
            IrType::Float => wasm![I32TruncSatF64S],
            IrType::String => {
                let func_index = func.registries().external_functions().register(
                    ("cast", "string2float".into()),
                    (vec![ValType::EXTERNREF], vec![ValType::F64]),
                )?;
                wasm![Call(func_index), I32TruncSatF64S]
            }
            IrType::QuasiInt => vec![],
            _ => hq_todo!("unimplemented cast: {:?} -> Int", from_base),
        },
        _ => hq_todo!("unimplemented cast: {:?} -> {:?}", from_base, target),
    })
}

pub fn acceptable_inputs(_fields: &Fields) -> Rc<[IrType]> {
    Rc::new([IrType::Number.or(IrType::String).or(IrType::Boolean)])
}

pub fn output_type(inputs: Rc<[IrType]>, &Fields(to): &Fields) -> HQResult<Option<IrType>> {
    Ok(Some(
        inputs[0]
            .base_types()
            .map(|from| best_cast_candidate(from, to))
            .collect::<HQResult<Vec<_>>>()?
            .into_iter()
            .reduce(IrType::or)
            .ok_or_else(|| make_hq_bug!("input was empty"))?,
    ))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {float; hq_cast; t @ super::Fields(IrType::Float)}
crate::instructions_test! {string; hq_cast; t @ super::Fields(IrType::String)}
crate::instructions_test! {int; hq_cast; t @ super::Fields(IrType::QuasiInt)}
