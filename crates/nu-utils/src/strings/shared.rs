#[allow(unused_imports)]
use crate::sformat;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{borrow::Borrow, fmt, ops::Deref};

/// A compact string with cheap shared clones.
#[repr(transparent)]
#[derive(Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SharedString(lean_string::LeanString);

impl SharedString {
    pub fn from_string(text: String) -> Self {
        Self(text.into())
    }
    pub const fn from_static_str(text: &'static str) -> Self {
        Self(lean_string::LeanString::from_static_str(text))
    }
    pub fn from_fmt(args: fmt::Arguments<'_>) -> Self {
        if let Some(text) = args.as_str() {
            Self::from_static_str(text)
        } else {
            use std::fmt::Write;
            let mut text = lean_string::LeanString::new();
            text.write_fmt(args).expect("string formatting failed");
            Self(text)
        }
    }
}
impl Deref for SharedString {
    type Target = str;
    fn deref(&self) -> &str {
        self.0.as_str()
    }
}
impl AsRef<str> for SharedString {
    fn as_ref(&self) -> &str {
        self
    }
}
impl Borrow<str> for SharedString {
    fn borrow(&self) -> &str {
        self
    }
}
impl fmt::Display for SharedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
impl fmt::Debug for SharedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
impl Serialize for SharedString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self)
    }
}
impl<'de> Deserialize<'de> for SharedString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from_string)
    }
}

/// Format a compact shared string.
#[macro_export]
macro_rules! sformat {
    ($($args:tt)*) => { $crate::strings::SharedString::from_fmt(format_args!($($args)*)) };
}

#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(size_of::<SharedString>() == 16);
    assert!(size_of::<Option<SharedString>>() == 16);
};

#[test]
fn macro_in_scope() {
    assert_eq!(&*sformat!("{}", 42), "42");
}
