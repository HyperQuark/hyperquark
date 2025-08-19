use crate::ir::Target as IrTarget;
use crate::prelude::*;
use crate::registry::SetRegistry;

pub type SpriteRegistry = SetRegistry<Rc<IrTarget>>;
