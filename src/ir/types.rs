use bitmask_enum::bitmask;

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

    StringNumber, // a string which can be interpreted as a non-nan number
    StringBoolean, // "true" or "false"
    StringNan, // some other string which can only be interpreted as NaN
    String = Self::StringNumber.or(Self::StringBoolean).or(Self::StringNan).bits,

    QuasiBoolean = Self::Boolean.or(Self::StringBoolean).bits,
    QuasiNumber = Self::Number.or(Self::StringNumber).bits,

    Any = Self::String.or(Self::Number).bits,

    Color,
}
