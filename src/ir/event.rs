// Ord is required to be used in a BTreeMap; Ord requires PartialOrd, Eq and PartialEq
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Event {
    FlagCLicked,
}
