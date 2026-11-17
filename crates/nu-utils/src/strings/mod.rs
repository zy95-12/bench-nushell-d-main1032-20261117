//! Compact immutable strings with shared or unique ownership.
mod shared;
mod unique;
pub use shared::SharedString;
pub use unique::UniqueString;

#[cfg(test)]
mod tests;
