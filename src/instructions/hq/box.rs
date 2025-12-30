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

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Any]))
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields { output_ty }: &Fields) -> HQResult<ReturnType> {
    Ok(Singleton(*output_ty))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
    _fields: &Fields,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

crate::instructions_test! {tests; hq_box; t @ super::Fields { output_ty: IrType::Any }}
