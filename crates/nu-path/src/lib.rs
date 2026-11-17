#![doc = include_str!("../README.md")]
mod assert_path_eq;
mod components;
pub mod dots;
pub mod expansions;
pub mod form;
mod helpers;
mod path;
mod tilde;
mod trailing_slash;

pub use components::components;
pub use expansions::{
    canonicalize_with, expand_path, expand_path_with, expand_to_real_path, locate_in_dirs,
};
pub use helpers::{cache_dir, data_dir, home_dir, nu_config_dir};
pub use path::*;
pub use tilde::expand_tilde;
pub use trailing_slash::{has_trailing_slash, strip_trailing_slash};

/// Whether a path names a Windows device rather than a regular filesystem entry.
pub fn is_windows_device_path(path: impl AsRef<std::path::Path>) -> bool {
    #[cfg(windows)]
    {
        matches!(path.as_ref().components().next(),
            Some(std::path::Component::Prefix(prefix))
                if matches!(prefix.kind(), std::path::Prefix::DeviceNS(_)))
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        false
    }
}

#[cfg(test)]
mod predictable_device_tests {
    use super::is_windows_device_path;
    #[test]
    fn predictable_device_paths_are_platform_specific() {
        assert_eq!(is_windows_device_path(r"\\.\NUL"), cfg!(windows));
        assert_eq!(is_windows_device_path(r"\\.\pipe\example"), cfg!(windows));
        assert!(!is_windows_device_path("normal/file"));
        assert!(!is_windows_device_path(r"C:\normal\file"));
        assert!(!is_windows_device_path(r"\\server\share\file"));
    }
}
