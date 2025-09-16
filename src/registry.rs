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

impl<K, V> RegistryType for MapRegistry<K, V>
where
    K: Hash + Eq + Clone,
{
    type Key = K;
    type Value = V;
}

impl<K, V> Registry for MapRegistry<K, V>
where
    K: Hash + Eq + Clone,
{
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

impl<K> RegistryType for SetRegistry<K>
where
    K: Hash + Eq + Clone,
{
    type Key = K;
    type Value = ();
}

impl<K> Registry for SetRegistry<K>
where
    K: Hash + Eq + Clone,
{
    fn registry(&self) -> &RefCell<IndexMap<K, ()>> {
        &self.0
    }
}

pub trait RegistryType {
    type Key: Hash + Eq + Clone = Box<str>;
    type Value;
}

pub trait Registry: Sized + RegistryType {
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

pub trait NamedRegistryItem<V> {
    const VALUE: V;
}
pub trait NamedRegistryItemOverride<V, A>: NamedRegistryItem<V> {
    fn r#override(arg: A) -> V;
}

pub trait NamedRegistrar<R>: RegistryType
where
    R: RegistryType,
{
    fn name<T>() -> R::Key;
}
impl<R> NamedRegistrar<R> for R
where
    R: RegistryType,
    R::Key: From<&'static str>,
{
    fn name<T>() -> R::Key {
        core::any::type_name::<T>().into()
    }
}

/// Registry type for items that will be registered with an arbitrary "name" of some sort which is `const`able.
///
/// This means we can store the value statically in a trait-implementing struct, and derive
/// the "name" from that struct (by default from the name of the struct).
///
/// A `NameRegistry` is defined with respect to a [`NamedRegistrar`], which can be any object
/// (probably a ZST) that implements [`RegistryType`], defining the types
/// [`Key`](RegistryType::Key) (defaults to [`Box<str>`]) and [`Value`](RegistryType::Value). Manually
/// implementing [`NamedRegistrar`] is only necessary if `Key: !From<&str>` or you wish to specify a
/// custom naming function.
///
/// To use a ZST `T` as an itemin a `NamedRegistry`, implement [`NamedRegistryItem<Value>`](NamedRegistryItem)
/// for `T` to define the default value for the item.
///
/// If you wish items in this registry to be overridable, `impl`
/// [`NamedRegistryItemOverride<Value, A>`](NamedRegistryItemOverride) `for T`, where `A` is an argument to
/// pass to the [`r#override`](NamedRegistryItemOverride::override) function. If multiple arguments are
/// needed, they should be wrapped into a tuple or struct.
///
/// # Examples
/// ```
/// # use hyperquark::prelude::*;
/// # fn main() -> HQResult<()> {
/// struct NumberRegistrar;
/// impl RegistryType for NumberRegistrar {
///     type Key = Box<str>;
///     type Value = u32;
/// }
/// // `NamedRegistrar` is implemented automatically because `Key = Box<str>`
///
/// type NumberRegistry = NamedRegistry<NumberRegistrar>;
///
/// struct Zero;
/// impl NamedRegistryItem<u32> for Zero {
///     const VALUE: u32 = 0;
/// }
///
/// struct Natural;
/// impl NamedRegistryItem<u32> for Natural {
///     const VALUE: u32 = 1;
/// }
/// impl NamedRegistryItemOverride<u32, u32> for Natural {
///     fn r#override(new: u32) -> u32 {
///         new
///     }
/// }
///
/// let num_reg = NumberRegistry::default();
/// let nat_pos: usize = num_reg.register::<Natural, _>()?;
/// let zero_pos: usize = num_reg.register::<Zero, _>()?;
/// let also_nat_pos: usize = num_reg.register_override::<Natural, _, _>(5)?;
/// assert_eq!(nat_pos, also_nat_pos);
///
/// // let also_zero_pos: usize = num_reg.register_override::<Zero, _, _>(0)?;
/// // ^ would fail to compile because `Zero` doesn't implement `NamedRegistryItemOverride`
///
/// # Ok(())
/// # }
/// ```
pub struct NamedRegistry<R>(MapRegistry<R::Key, R::Value>)
where
    R: NamedRegistrar<R>;

impl<R> NamedRegistry<R>
where
    R: NamedRegistrar<R>,
{
    pub fn registry(&self) -> &RefCell<IndexMap<R::Key, R::Value>> {
        self.0.registry()
    }

    pub fn register<T, N>(&self) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
        T: NamedRegistryItem<R::Value>,
    {
        self.0.register(R::name::<T>(), T::VALUE)
    }

    pub fn register_override<T, N, A>(&self, override_arg: A) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
        T: NamedRegistryItem<R::Value> + NamedRegistryItemOverride<R::Value, A>,
    {
        self.0
            .register_override(R::name::<T>(), T::r#override(override_arg))
    }
}

impl<R> Default for NamedRegistry<R>
where
    R: NamedRegistrar<R>,
{
    fn default() -> Self {
        Self(MapRegistry::default())
    }
}

impl<R> Clone for NamedRegistry<R>
where
    R: NamedRegistrar<R>,
    R::Value: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
