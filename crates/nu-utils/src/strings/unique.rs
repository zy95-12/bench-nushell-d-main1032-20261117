#[allow(unused_imports)]
use crate::uformat;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{borrow::Borrow, fmt, ops::Deref};

/// A compact string with unique, non-reference-counted heap storage.
#[repr(transparent)]
#[derive(Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct UniqueString(byteyarn::Yarn);

impl UniqueString {
    pub fn from_string(text: String) -> Self {
        Self(text.into())
    }
    pub const fn from_static_str(text: &'static str) -> Self {
        Self(byteyarn::Yarn::from_static(text))
    }
    pub fn from_fmt(args: fmt::Arguments<'_>) -> Self {
        Self(byteyarn::Yarn::from_fmt(args))
    }
}
impl Deref for UniqueString {
    type Target = str;
    fn deref(&self) -> &str {
        self.0.as_str()
    }
}
impl AsRef<str> for UniqueString {
    fn as_ref(&self) -> &str {
        self
    }
}
impl Borrow<str> for UniqueString {
    fn borrow(&self) -> &str {
        self
    }
}
impl fmt::Display for UniqueString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
impl fmt::Debug for UniqueString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
impl Serialize for UniqueString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self)
    }
}
impl<'de> Deserialize<'de> for UniqueString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from_string)
    }
}

/// Format a compact unique string.
#[macro_export]
macro_rules! uformat {
    ($($args:tt)*) => { $crate::strings::UniqueString::from_fmt(format_args!($($args)*)) };
}

#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(size_of::<UniqueString>() == 16);
    assert!(size_of::<Option<UniqueString>>() == 16);
};

#[test]
fn macro_in_scope() {
    assert_eq!(&*uformat!("{}", 42), "42");
}
