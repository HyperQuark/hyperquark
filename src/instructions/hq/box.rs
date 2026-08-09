use super::super::prelude::*;

#[derive(Clone, Debug)]
pub struct Fields {
    pub output_ty: IrType,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            r#"{{
            "output_ty": "{}"
        }}"#,
            self.output_ty
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    _fields: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let t = inputs[0];
    Ok(wasm![
        @boxed(t)
    ])
}

pub fn acceptable_inputs(Fields { output_ty }: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([*output_ty]))
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields { output_ty }: &Fields) -> HQResult<ReturnType> {
    Ok(Singleton(*output_ty))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub fn const_fold(
    inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
    Fields { output_ty }: &Fields,
) -> HQResult<ConstFold> {
    Ok(match &inputs[0] {
        ConstFoldItem::Basic(val) => {
            ConstFold::Folded(Rc::from([ConstFoldItem::Boxed(val.clone(), *output_ty)]))
        }
        ConstFoldItem::Boxed(..) | ConstFoldItem::Stack(_) | ConstFoldItem::Unknown { .. } => {
            ConstFold::NotFoldable
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::instructions::tests::assert_valid_json;

    #[test]
    fn fields_display_is_valid_json() {
        assert_valid_json(format!("{}", Fields {
            output_ty: IrType::Any,
        }));
    }
}

crate::instructions_test! (
    mod tests for hq_box(t) {
        fields = super::Fields { output_ty: IrType::Any };
    }
);
