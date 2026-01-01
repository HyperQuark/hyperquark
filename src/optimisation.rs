use crate::prelude::*;
use crate::wasm::flags::Switch;
use crate::{ir::IrProject, wasm::WasmFlags};

mod const_folding;
mod loop_unrolling;
mod ssa;

pub use const_folding::{ConstFold, ConstFoldItem, ConstFoldState};
pub use ssa::SSAToken;

pub fn ir_optimise(ir: &Rc<IrProject>, flags: &WasmFlags) -> HQResult<SSAToken> {
    loop_unrolling::unroll_loops(ir, flags)?;

    if flags.print_ir == Switch::On {
        crate::log("ir (after loop unrolling):");
        crate::log(format!("{ir}").as_str());
    }

    let ssa_token = ssa::optimise_variables(ir)?;

    if flags.print_ir == Switch::On {
        crate::log("ir (after SSA):");
        crate::log(format!("{ir}").as_str());
    }

    const_folding::const_fold(ir, ssa_token)?;

    Ok(ssa_token)
}
