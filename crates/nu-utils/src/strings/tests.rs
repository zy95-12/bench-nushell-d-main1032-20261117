use super::*;
use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
    ops::Deref,
};

fn traits<
    T: Clone
        + Debug
        + Default
        + Display
        + Deref<Target = str>
        + Eq
        + Hash
        + Ord
        + PartialEq
        + PartialOrd
        + AsRef<str>
        + Borrow<str>,
>() {
}

#[test]
fn round_trip_and_traits() {
    traits::<SharedString>();
    traits::<UniqueString>();
    for text in [
        "",
        "short",
        "🦀unicode",
        "a long dynamically allocated string exceeding inline storage",
    ] {
        let shared = SharedString::from_string(text.to_owned());
        let unique = UniqueString::from_string(text.to_owned());
        assert_eq!(&*shared, text);
        assert_eq!(&*unique, text);
        assert_eq!(shared.to_string(), text);
        assert_eq!(unique.to_string(), text);
        assert_eq!(format!("{shared:?}"), format!("{text:?}"));
        assert_eq!(format!("{unique:?}"), format!("{text:?}"));
        let json = serde_json::to_string(text).unwrap();
        assert_eq!(serde_json::to_string(&shared).unwrap(), json);
        assert_eq!(serde_json::to_string(&unique).unwrap(), json);
        assert_eq!(serde_json::from_str::<SharedString>(&json).unwrap(), shared);
        assert_eq!(serde_json::from_str::<UniqueString>(&json).unwrap(), unique);
        let mut map = HashMap::new();
        map.insert(shared.clone(), 1);
        assert_eq!(map.get(text), Some(&1));
        let mut map = HashMap::new();
        map.insert(unique.clone(), 2);
        assert_eq!(map.get(text), Some(&2));
    }
    assert!(SharedString::from_static_str("a") < SharedString::from_static_str("b"));
    assert!(UniqueString::from_static_str("a") < UniqueString::from_static_str("b"));
    assert!(SharedString::default().is_empty());
    assert!(UniqueString::default().is_empty());
    assert_eq!(&*crate::sformat!("answer {}", 42), "answer 42");
    assert_eq!(&*crate::uformat!("answer {}", 42), "answer 42");
}

#[test]
fn static_and_clone_storage() {
    const TEXT: &str = "a static literal long enough to exceed small string storage";
    let shared = SharedString::from_static_str(TEXT);
    let unique = UniqueString::from_static_str(TEXT);
    assert_eq!(shared.as_ptr(), TEXT.as_ptr());
    assert_eq!(unique.as_ptr(), TEXT.as_ptr());
    assert_eq!(shared.clone().as_ptr(), TEXT.as_ptr());
    assert_eq!(unique.clone().as_ptr(), TEXT.as_ptr());
    let shared = SharedString::from_string(TEXT.to_owned());
    let cloned = shared.clone();
    assert_eq!(shared.as_ptr(), cloned.as_ptr());
    drop(shared);
    assert_eq!(&*cloned, TEXT);
    let unique = UniqueString::from_string(TEXT.to_owned());
    let cloned = unique.clone();
    assert_ne!(unique.as_ptr(), cloned.as_ptr());
    drop(unique);
    assert_eq!(&*cloned, TEXT);
}

#[test]
fn small_strings_live_inline() {
    let shared = SharedString::from_string("small".to_owned());
    let unique = UniqueString::from_string("small".to_owned());
    for (start, size, ptr) in [
        (
            &shared as *const _ as usize,
            size_of::<SharedString>(),
            shared.as_ptr() as usize,
        ),
        (
            &unique as *const _ as usize,
            size_of::<UniqueString>(),
            unique.as_ptr() as usize,
        ),
    ] {
        assert!((start..start + size).contains(&ptr));
    }
}
