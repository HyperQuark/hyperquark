use crate::prelude::*;
use crate::sb3::Sb3Project;

pub struct IrProject {
    
}

impl TryFrom<Sb3Project> for IrProject {
    type Error = HQError;

    fn try_from(sb3: Sb3Project) -> HQResult<Self> {
        hq_todo!()
    }
}