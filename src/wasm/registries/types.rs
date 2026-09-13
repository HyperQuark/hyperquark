use wasm_encoder::HeapType;

use crate::prelude::*;

mod dependencies;
mod registration;
#[macro_use]
mod rec_group;
mod registry;
mod subtypes;
mod tyfp;

pub use rec_group::*;
pub use registration::*;
pub use registry::{CompoundType, RecGroup, RegistryItem, TypeRegistry};
pub use subtypes::*;
pub use tyfp::*;

rec_group! {
    rec_grp {
        TStepFunc = TFunc{
            {(
                {TNonNullable{{TStackArray}}},
                {TNullable{{TStructRef}}},
            )},
            {()}
        };
        TStackStruct = TStruct{{(
            {TMutField{{TNonNullable{{TStepFunc}}}}},
            {TConstField{{TNullable{{TStructRef}}}}},
        )}};
        TStackArray = TDynArray{{TNullable{{TStackStruct}}}};
    }
}

pub type TDynArrayField<T> = TArray<TMutField<T>>;
pub type TDynArray<T> = TStruct<(
    TMutField<TNonNullable<TDynArrayField<T>>>,
    (TMutField<TI32>, ()),
)>;

pub type TThreadArray = TDynArray<TNullable<TStackArray>>;

pub type TTargetThreadsStruct =
    TStruct<(TMutField<TI32>, (TMutField<TNonNullable<TThreadArray>>, ()))>;

pub type TTargetThreadArray = TArray<TMutField<TNonNullable<TTargetThreadsStruct>>>;
