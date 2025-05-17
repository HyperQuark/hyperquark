use crate::prelude::*;
use core::hash::Hash;

#[derive(Clone)]
pub struct MapRegistry<K, V>(RefCell<IndexMap<K, V>>)
where
    K: Hash + Eq + Clone;

// deriving Default doesn't work if V: !Default, so we implement it manually
impl<K, V> Default for MapRegistry<K, V>
where
    K: Hash + Eq + Clone,
{
    fn default() -> Self {
        Self(RefCell::new(IndexMap::default()))
    }
}

impl<K, V> Registry for MapRegistry<K, V>
where
    K: Hash + Eq + Clone,
{
    type Key = K;
    type Value = V;

    fn registry(&self) -> &RefCell<IndexMap<K, V>> {
        &self.0
    }
}

#[derive(Clone)]
pub struct SetRegistry<K>(RefCell<IndexMap<K, ()>>)
where
    K: Hash + Eq + Clone;

impl<K> Default for SetRegistry<K>
where
    K: Hash + Eq + Clone,
{
    fn default() -> Self {
        Self(RefCell::new(IndexMap::default()))
    }
}

impl<K> Registry for SetRegistry<K>
where
    K: Hash + Eq + Clone,
{
    type Key = K;
    type Value = ();

    fn registry(&self) -> &RefCell<IndexMap<K, ()>> {
        &self.0
    }
}

pub trait Registry: Sized {
    const IS_SET: bool = false;

    type Key: Hash + Clone + Eq;
    type Value;

    fn registry(&self) -> &RefCell<IndexMap<Self::Key, Self::Value>>;

    /// get the index of the specified item, inserting it if it doesn't exist in the map already.
    /// Doesn't check if the provided value matches what's already there. This is generic because
    /// various different numeric types are needed in different places, so it's easiest to encapsulate
    /// the casting logic in here.
    fn register<N>(&self, key: Self::Key, value: Self::Value) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.registry()
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .entry(key.clone())
            .or_insert(value);
        N::try_from(
            self.registry()
                .try_borrow()?
                .get_index_of(&key)
                .ok_or_else(|| make_hq_bug!("couldn't find entry in Registry"))?,
        )
        .map_err(|_| make_hq_bug!("registry item index out of bounds"))
    }

    fn register_override<N>(&self, key: Self::Key, value: Self::Value) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.registry()
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .entry(key.clone())
            .insert_entry(value);
        N::try_from(
            self.registry()
                .try_borrow()?
                .get_index_of(&key)
                .ok_or_else(|| make_hq_bug!("couldn't find entry in Registry"))?,
        )
        .map_err(|_| make_hq_bug!("registry item index out of bounds"))
    }
}

pub trait RegistryDefault: Registry<Value: Default> {
    fn register_default<N>(&self, key: Self::Key) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.register(key, Self::Value::default())
    }
}

impl<R> RegistryDefault for R where R: Registry<Value: Default> {}
