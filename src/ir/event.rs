// Ord is required to be used in a BTreeMap; Ord requires PartialOrd, Eq and PartialEq
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Event {
    FlagClicked,
    Broadcast(Box<str>),
    SpriteClicked(u32),
}
