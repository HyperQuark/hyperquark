#![allow(clippy::allow_attributes_without_reason)]
#![allow(clippy::allow_attributes)]
#![allow(dead_code)]

macro_rules! size_from_type {
    (i8) => {{ 1 }};
    (i16) => {{ 2 }};
    (i32) => {{ 4 }};
    (f32) => {{ 4 }};
    (i64) => {{ 8 }};
    (f64) => {{ 8 }};
}

macro_rules! memory_layout {
    {
        $mod:ident
        $(#[$attr:meta] $name:ident : $ty:tt)+
    } => {
        memory_layout!(
            $mod
            $(#[$attr] $name : $ty)+
            @ 0;
        );
    };

    {
        $mod:ident
        #[$attr:meta] $name:ident : $ty:tt
        $(#[$attr2:meta] $name2:ident : $ty2:tt)+
        @ $tot:expr;
        $(#[$attr3:meta] $name3:ident $tot3:expr;)*
    } => {
        memory_layout!(
            $mod
            $(#[$attr2] $name2 : $ty2)+
            @ $tot + size_from_type!($ty);
            $(#[$attr3] $name3 $tot3;)*
            #[$attr] $name $tot;
        );
    };
    ($mod:ident #[$attr:meta] $name:ident : $ty:tt @ $tot:expr; $(#[$attr3:meta] $name3:ident $tot3:expr;)*) => {
        pub mod $mod {
            $(#[$attr3] pub const $name3: u32 = $tot3;)*
            #[$attr] pub const $name: u32 = $tot;

            pub const BLOCK_SIZE: u32 = $tot + size_from_type!($ty);
        }
    }
}

memory_layout! {
    stage
    /// Backdrop number of stage (i32)
    COSTUME: i32
    /// 4-byte adding (so that sprite chunks are aligned to 8 bits)
    _PADDING: i32
}

memory_layout! {
    sprite
    /// x-position of sprite (f64)
    X: f64
    /// y-position of sprite (f64)
    Y: f64
    /// hue component of pen colour (0-100) (f32)
    PEN_COLOR: f32
    /// saturation component of pen colour (0-100) (f32)
    PEN_SATURATION: f32
    /// value component of pen colour (0-100) (f32)
    PEN_BRIGHTNESS: f32
    /// transparency of pen (f32)
    PEN_TRANSPARENCY: f32
    /// red component of rgba representation of pen colour (0-1) (f32)
    PEN_COLOR_R: f32
    /// green component of rgba representation of pen colour (0-1) (f32)
    PEN_COLOR_G: f32
    /// blue component of rgba representation of pen colour (0-1) (f32)
    PEN_COLOR_B: f32
    /// alpha component of rgba representation of pen colour (0-1) (f32)
    PEN_COLOR_A: f32
    /// radius of pen (f64)
    PEN_SIZE: f64
    /// non-zero if pen down, 0 otherwise (i8)
    PEN_DOWN: i8
    /// non-zero if sprite is visible, 0 otherwise (i8)
    VISIBLE: i8
    /// bytes 58-59 padding
    _PADDING: i16
    /// current costume number, 0-indexed (i32)
    COSTUME: i32
    /// sprite size, where default is 100(%) (f64)
    SIZE: f64
    /// sprite rotation, in scratch angles (0 = up, 90 = right) (f64)
    ROTATION: f64
}
