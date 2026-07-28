use core::fmt;

// Ord is required to be used in a BTreeMap; Ord requires PartialOrd, Eq and PartialEq
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Event {
    FlagClicked,
    Broadcast(Box<str>),
    SpriteClicked(u32),
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FlagClicked => write!(f, "FlagClicked"),
            Self::Broadcast(name) => write!(f, "Broadcast({name})"),
            Self::SpriteClicked(idx) => write!(f, "SpriteClicked({idx})"),
        }
    }
}
