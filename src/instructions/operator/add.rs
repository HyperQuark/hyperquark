use crate::ir::types::Type as IrType;
use crate::prelude::*;

use wasm_encoder::Instruction;

pub fn instructions(t1: IrType, t2: IrType) -> HQResult<Vec<Instruction<'static>>> {
    hq_todo!();
    if IrType::QuasiInt.contains(t1) {
        if IrType::QuasiInt.contains(t2) {
            hq_todo!()
        } else if IrType::Float.contains(t2) {
            hq_todo!()
        } else {
            hq_todo!()
        }
    } else if IrType::Float.contains(t2) {
        hq_todo!()
    } else {
        hq_todo!()
    }
}

pub fn output_type(t1: IrType, t2: IrType) -> HQResult<IrType> {
    hq_todo!();
}

// (context (param $MEMORY_LOCATION i32) (result i32)
//     (local $EXTERNREF ref.extern)
//     (local $F64 f64)
//     (local $I64 i64)
//     (local $I32 i32)
//     (local $I32_2 i32)
//     (local $F64_2 i32)
//     (inline $add_QuasiInteger_QuasiInteger (stack i64 i64) (result i64)
//             i64.add)
//     (inline $add_QuasiInteger_Float (stack i64 f64) (result f64)
//         local.set $F64
//         f64.convert_i64_s
//         local.get $F64
//         f64.add
//     )
//     (inline $add_Float_QuasiInteger (stack f64 i64) (result f64)
//         f64.convert_i64_s
//         f64.add
//     )
//     (inline $add_Float_Float (stack f64 f64) (result f64)
//         f64.convert_i64_s
//         f64.add
//     )
// )

crate::instructions_test!(t1, t2);