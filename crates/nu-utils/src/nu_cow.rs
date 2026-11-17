use serde::{Deserialize, Deserializer, Serialize};
use std::{borrow::Borrow, fmt, ops::Deref};

/// A borrowed or owned value with independently chosen concrete storage types.
#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum NuCow<B, O> {
    Borrowed(B),
    Owned(O),
}

impl<'de, B, O: Deserialize<'de>> Deserialize<'de> for NuCow<B, O> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        O::deserialize(deserializer).map(Self::Owned)
    }
}

impl<B: Deref, O: Borrow<B::Target>> Deref for NuCow<B, O> {
    type Target = B::Target;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(value) => value,
            Self::Owned(value) => value.borrow(),
        }
    }
}

impl<B: Deref, O: Borrow<B::Target>> PartialEq for NuCow<B, O>
where
    B::Target: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl<B: Deref, O: Borrow<B::Target>> Eq for NuCow<B, O> where B::Target: Eq {}

impl<B: fmt::Debug, O: fmt::Debug> fmt::Debug for NuCow<B, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Borrowed(value) if f.alternate() => {
                f.debug_tuple("Borrowed").field(value).finish()
            }
            Self::Owned(value) if f.alternate() => f.debug_tuple("Owned").field(value).finish(),
            Self::Borrowed(value) => value.fmt(f),
            Self::Owned(value) => value.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn values_and_serde() {
        let borrowed: NuCow<&str, String> = NuCow::Borrowed("hello");
        let owned = NuCow::Owned(String::from("hello"));
        assert_eq!(borrowed, owned);
        assert_eq!(borrowed.clone(), owned.clone());
        assert_eq!(format!("{borrowed:?}"), "\"hello\"");
        assert!(format!("{borrowed:#?}").starts_with("Borrowed("));
        assert!(format!("{owned:#?}").starts_with("Owned("));
        assert_eq!(serde_json::to_string(&borrowed).unwrap(), "\"hello\"");
        assert_eq!(serde_json::to_string(&owned).unwrap(), "\"hello\"");
        let decoded: NuCow<&str, String> = serde_json::from_str("\"hello\"").unwrap();
        assert!(matches!(decoded, NuCow::Owned(_)));
        assert_eq!(decoded, borrowed);
    }
    #[test]
    fn deserialize_does_not_require_borrowed_deserialize() {
        struct NoDeserialize;
        let decoded: NuCow<NoDeserialize, Vec<u32>> = serde_json::from_str("[1,2]").unwrap();
        assert!(matches!(decoded, NuCow::Owned(v) if v == [1,2]));
    }
}
