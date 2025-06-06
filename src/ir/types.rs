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
    pub const BASE_TYPES: [Self; 3] = [Self::String, Self::QuasiInt, Self::Float];

    pub fn is_base_type(self) -> bool {
        (!self.is_none()) && Self::BASE_TYPES.iter().any(|ty| ty.contains(self))
    }

    pub fn base_type(self) -> Option<Self> {
        if !self.is_base_type() {
            return None;
        }
        Self::BASE_TYPES
            .iter()
            .copied()
            .find(|&ty| ty.contains(self))
    }

    pub fn base_types(self) -> Box<dyn Iterator<Item = Self>> {
        if self.is_none() {
            return Box::new(core::iter::empty());
        }
        Box::new(
            Self::BASE_TYPES
                .iter()
                .filter(move |ty| ty.intersects(self))
                .copied(),
        )
    }

    pub const fn maybe_positive(self) -> bool {
        self.contains(Self::IntPos)
            || self.intersects(Self::FloatPos)
            || self.contains(Self::BooleanTrue)
    }

    pub const fn maybe_negative(self) -> bool {
        self.contains(Self::IntNeg) || self.intersects(Self::FloatNeg)
    }

    pub const fn maybe_zero(self) -> bool {
        self.contains(Self::IntZero)
            || self.contains(Self::BooleanFalse)
            || self.intersects(Self::FloatZero)
    }

    pub const fn maybe_nan(self) -> bool {
        self.intersects(Self::FloatNan) || self.contains(Self::StringNan)
    }

    pub const fn none_if_false(condition: bool, if_true: Self) -> Self {
        if condition {
            if_true
        } else {
            Self::none()
        }
    }
}
