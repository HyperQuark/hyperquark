//! Functional programming in the type system. Or something like that.

pub trait List {
    type Head;
    type Tail: List;

    type Concat<Other: List>: List;
}

impl List for () {
    type Head = !;
    type Tail = ();

    type Concat<Other: List> = Other;
}

impl<Head, Tail> List for (Head, Tail)
where
    Tail: List,
{
    type Head = Head;
    type Tail = Tail;

    type Concat<Other: List> = (Head, Tail::Concat<Other>);
}

pub trait ListLen {
    const LEN: usize;
}

impl ListLen for () {
    const LEN: usize = 0;
}

impl<Head, Tail> ListLen for (Head, Tail)
where
    Tail: ListLen,
{
    const LEN: usize = Tail::LEN + 1;
}

pub trait Reverse: List {
    type Reversed: List;
}

impl Reverse for () {
    type Reversed = ();
}

impl<Head, Tail> Reverse for (Head, Tail)
where
    Tail: Reverse,
{
    type Reversed = <Tail::Reversed as List>::Concat<(Head, ())>;
}

pub trait Bool {
    const BOOL: bool;
}

pub trait Func {
    type Func<T>;
}

pub trait Filter<Cond: Func>: List {
    type Filtered: List;
}

impl<Cond: Func> Filter<Cond> for () {
    type Filtered = ();
}

pub trait FilterResult<Cond: Func, const HEAD: bool> {
    type FilterResult: List;
}

impl<Cond, Head, Tail> FilterResult<Cond, true> for (Head, Tail)
where
    Tail: Filter<Cond>,
    Cond: Func,
{
    type FilterResult = (Head, Tail::Filtered);
}

impl<Cond, Head, Tail> FilterResult<Cond, false> for (Head, Tail)
where
    Tail: Filter<Cond>,
    Cond: Func,
{
    type FilterResult = Tail::Filtered;
}

impl<Cond, Head, Tail> Filter<Cond> for (Head, Tail)
where
    Tail: Filter<Cond>,
    Cond: Func,
    Cond::Func<Head>: Bool,
    (Head, Tail): FilterResult<Cond, { <Cond::Func<Head> as Bool>::BOOL }>,
{
    type Filtered =
        <(Head, Tail) as FilterResult<Cond, { <Cond::Func<Head> as Bool>::BOOL }>>::FilterResult;
}

pub trait All<Cond: Func>: List {
    const ALL: bool;
}

impl<Cond: Func> All<Cond> for () {
    const ALL: bool = true;
}

impl<Cond, Head, Tail> All<Cond> for (Head, Tail)
where
    Cond: Func,
    Cond::Func<Head>: Bool,
    Tail: All<Cond>,
{
    const ALL: bool = <Cond::Func<Head> as Bool>::BOOL && <Tail as All<Cond>>::ALL;
}

pub trait Map<F: Func>: List {
    type Mapped: List;
}

impl<F: Func> Map<F> for () {
    type Mapped = ();
}

impl<F: Func, Head, Tail: Map<F>> Map<F> for (Head, Tail) {
    type Mapped = (F::Func<Head>, Tail::Mapped);
}

pub trait ListItem<const I: usize> {
    type Get;
}

impl<const I: usize> ListItem<I> for () {
    type Get = !;
}

impl<Head, Tail> ListItem<0> for (Head, Tail) {
    type Get = Head;
}

pub struct ConstSat<const SAT: bool>;

pub trait True {}

impl True for ConstSat<true> {}

impl<const I: usize, Head, Tail> ListItem<I> for (Head, Tail)
where
    ConstSat<{ I > 0 }>: True,
    Tail: ListItem<{ I - 1 }>,
{
    type Get = Tail::Get;
}
