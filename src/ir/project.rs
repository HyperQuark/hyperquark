use super::{Target, Thread};
use crate::prelude::*;
use crate::sb3::Sb3Project;

pub struct IrProject {
    threads: Box<[Thread]>,
}

impl IrProject {
    pub fn threads(&self) -> &[Thread] {
        self.threads.borrow()
    }
}

impl TryFrom<Sb3Project> for IrProject {
    type Error = HQError;

    fn try_from(sb3: Sb3Project) -> HQResult<Self> {
        let threads = sb3
            .targets
            .iter()
            .map(|target| {
                let ir_target = Rc::new(Target::new(target.name.clone(), target.is_stage));
                let blocks = &target.blocks;
                blocks
                    .values()
                    .filter_map(|block| {
                        let thread =
                            Thread::try_from_top_block(block, blocks, Rc::clone(&ir_target))
                                .transpose()?;
                        Some(thread)
                    })
                    .collect::<HQResult<Box<[_]>>>()
            })
            .collect::<HQResult<Box<[_]>>>()?
            .iter()
            .flatten()
            .cloned()
            .collect::<Box<[_]>>();
        Ok(IrProject { threads })
    }
}
