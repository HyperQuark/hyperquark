use alloc::rc::{Rc as CoreRc, Weak as CoreWeak};
use core::cmp::{Eq, PartialEq};
use core::fmt::{Debug, Display};
use core::hash::Hash;
use core::iter::FromIterator;
use core::ops::Deref;

pub struct Rc<T>(CoreRc<T>)
where
    T: ?Sized;

impl<T> Rc<T> {
    pub fn new(value: T) -> Self {
        Self(CoreRc::new(value))
    }

    #[must_use]
    pub fn downgrade(this: &Self) -> Weak<T> {
        Weak(CoreRc::downgrade(&this.0))
    }

    #[must_use]
    pub fn as_ptr(this: &Self) -> *const T {
        CoreRc::as_ptr(&this.0)
    }

    #[must_use]
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        CoreRc::ptr_eq(&this.0, &other.0)
    }
}

impl<T> AsRef<T> for Rc<T> {
    fn as_ref(&self) -> &T {
        self.0.as_ref()
    }
}

impl<T> FromIterator<T> for Rc<[T]> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(CoreRc::from_iter(iter))
    }
}

impl<T> Clone for Rc<T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        Self(CoreRc::clone(&self.0))
    }
}

impl<T> Deref for Rc<T>
where
    T: ?Sized,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Default for Rc<T>
where
    T: Default,
{
    fn default() -> Self {
        Self(CoreRc::default())
    }
}

impl<T> Debug for Rc<T>
where
    T: Debug + ?Sized,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<T> Display for Rc<T>
where
    T: Display + ?Sized,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<T> PartialEq for Rc<T>
where
    T: PartialEq + ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.0, &other.0)
    }
}

impl<T> PartialOrd for Rc<T>
where
    T: PartialOrd + ?Sized,
{
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        PartialOrd::partial_cmp(&self.0, &other.0)
    }
}

impl<T> Ord for Rc<T> where T: Ord {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        Ord::cmp(&self.0, &other.0)
    }
}

impl<T> Eq for Rc<T> where T: Eq {}

impl<U, T> From<U> for Rc<T>
where
    CoreRc<T>: From<U>,
    T: ?Sized,
{
    fn from(value: U) -> Self {
        Self(value.into())
    }
}

impl<T> Hash for Rc<T>
where
    T: Hash + ?Sized,
{
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(&self.0, state);
    }
}

pub struct Weak<T>(CoreWeak<T>)
where
    T: ?Sized;

impl<T> Weak<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self(CoreWeak::new())
    }

    #[must_use]
    pub fn upgrade(&self) -> Option<Rc<T>> {
        self.0.upgrade().map(|rc| Rc(rc))
    }
}

impl<T> Clone for Weak<T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        Self(CoreWeak::clone(&self.0))
    }
}

impl<T> Debug for Weak<T>
where
    T: Debug + ?Sized,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<T> Default for Weak<T> {
    fn default() -> Self {
        Self::new()
    }
}
