use nu_protocol::ShellError;

/// uucore's localizer is a per-thread OnceLock for a single utility. Run each
/// call on a fresh scoped thread so consecutive shell commands can use different
/// utilities without inheriting the first utility's message catalog.
pub(crate) fn with_localized<T: Send>(
    utility: &str,
    run: impl FnOnce() -> T + Send,
) -> Result<T, ShellError> {
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                uucore::locale::setup_localization(utility).map_err(|err| {
                    ShellError::GenericError {
                        error: "Could not initialize coreutils messages".into(),
                        msg: err.to_string(),
                        span: None,
                        help: None,
                        inner: vec![],
                    }
                })?;
                Ok(run())
            })
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consecutive_utilities_have_their_own_catalogs() {
        for (utility, key, expected) in [
            ("cp", "cp-error-same-file", "are the same file"),
            ("mv", "mv-error-self-target-subdirectory", "cannot move"),
            (
                "mkdir",
                "mkdir-verbose-created-directory",
                "created directory",
            ),
            ("cp", "cp-error-same-file", "are the same file"),
        ] {
            let message = with_localized(utility, || uucore::locale::get_message(key)).unwrap();
            assert!(message.contains(expected), "{utility}: {message}");
        }
    }
}
