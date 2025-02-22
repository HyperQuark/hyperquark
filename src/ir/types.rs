use crate::prelude::*;
use bitmask_enum::bitmask;

/// a bitmask of possible IR types
#[bitmask(u32)]
#[bitmask_config(vec_debug, flags_iter)]
pub enum Type {
    IntZero,
    IntPos,
    IntNeg,
    IntNonZero = Self::IntPos.or(Self::IntNeg).bits,
    Int = Self::IntNonZero.or(Self::IntZero).bits,

    FloatPosZero,
    FloatNegZero,
    FloatZero = Self::FloatPosZero.or(Self::FloatNegZero).bits,

    FloatPosInt,
    FloatPosFrac,
    FloatPosReal = Self::FloatPosInt.or(Self::FloatPosFrac).bits,

    FloatNegInt,
    FloatNegFrac,
    FloatNegReal = Self::FloatNegInt.or(Self::FloatNegFrac).bits,

    FloatPosInf,
    FloatNegInf,
    FloatInf = Self::FloatPosInf.or(Self::FloatNegInf).bits,

    FloatNan,

    FloatPos = Self::FloatPosReal.or(Self::FloatPosInf).bits,
    FloatNeg = Self::FloatNegReal.or(Self::FloatNegInf).bits,

    FloatPosWhole = Self::FloatPosInt.or(Self::FloatPosZero).bits,
    FloatNegWhole = Self::FloatNegInt.or(Self::FloatNegZero).bits,

    FloatInt = Self::FloatPosWhole.or(Self::FloatNegWhole).bits,
    FloatFrac = Self::FloatPosFrac.or(Self::FloatNegFrac).bits,
    FloatReal = Self::FloatInt.or(Self::FloatFrac).bits,
    FloatNotNan = Self::FloatReal.or(Self::FloatInf).bits,
    Float = Self::FloatNotNan.or(Self::FloatNan).bits,

    BooleanTrue,
    BooleanFalse,
    Boolean = Self::BooleanTrue.or(Self::BooleanFalse).bits,

    QuasiInt = Self::Int.or(Self::Boolean).bits,

    Number = Self::QuasiInt.or(Self::Float).bits,

    StringNumber,  // a string which can be interpreted as a non-nan number
    StringBoolean, // "true" or "false"
    StringNan,     // some other string which can only be interpreted as NaN
    String = Self::StringNumber
        .or(Self::StringBoolean)
        .or(Self::StringNan)
        .bits,

    QuasiBoolean = Self::Boolean.or(Self::StringBoolean).bits,
    QuasiNumber = Self::Number.or(Self::StringNumber).bits,

    Any = Self::String.or(Self::Number).bits,

    Color,
}
impl Type {
    pub const BASE_TYPES: [Type; 3] = [Type::String, Type::QuasiInt, Type::Float];

    pub fn is_base_type(&self) -> bool {
        (!self.is_none()) && Type::BASE_TYPES.iter().any(|ty| ty.contains(*self))
    }

    pub fn base_type(&self) -> Option<Type> {
        if !self.is_base_type() {
            return None;
        }
        Type::BASE_TYPES
            .iter()
            .cloned()
            .find(|&ty| ty.contains(*self))
    }

    pub fn base_types(&self) -> Box<dyn Iterator<Item = &Type> + '_> {
        if self.is_none() {
            return Box::new(core::iter::empty());
        }
        Box::new(Type::BASE_TYPES.iter().filter(|ty| ty.intersects(*self)))
    }

    pub fn maybe_positive(&self) -> bool {
        self.contains(Type::IntPos)
            || self.intersects(Type::FloatPos)
            || self.contains(Type::BooleanTrue)
    }

    pub fn maybe_negative(&self) -> bool {
        self.contains(Type::IntNeg) || self.intersects(Type::FloatNeg)
    }

    pub fn maybe_zero(&self) -> bool {
        self.contains(Type::IntZero)
            || self.contains(Type::BooleanFalse)
            || self.intersects(Type::FloatZero)
    }

    pub fn maybe_nan(&self) -> bool {
        self.intersects(Type::FloatNan) || self.contains(Type::StringNan)
    }

    pub fn none_if_false(condition: bool, if_true: Type) -> Type {
        if condition {
            if_true
        } else {
            Type::none()
        }
    }
}
