use std::ops::Deref;

/// A reference originating from either an outer or a local lifetime.
pub enum MultiLife<'out: 'local, 'local, T: ?Sized> {
    Out(&'out T),
    Local(&'local T),
}

impl<T: ?Sized> Deref for MultiLife<'_, '_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        match self {
            Self::Out(x) => x,
            Self::Local(x) => x,
        }
    }
}

#[test]
fn deref() {
    let outer = String::from("outer");
    assert_eq!(&*MultiLife::Out(outer.as_str()), "outer");
    let local = String::from("local");
    assert_eq!(&*MultiLife::Local(local.as_str()), "local");
}
