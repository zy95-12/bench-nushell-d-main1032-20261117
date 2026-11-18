use crate::*;

/// Enforce explicit variable type annotations on initialization and reassignment.
/// Inferred variable types keep their ordinary advisory behavior.
pub static ENFORCE_RUNTIME_ANNOTATIONS: ExperimentalOption =
    ExperimentalOption::new(&EnforceRuntimeAnnotations);
struct EnforceRuntimeAnnotations;
impl ExperimentalOptionMarker for EnforceRuntimeAnnotations {
    const IDENTIFIER: &'static str = "enforce-runtime-annotations";
    const DESCRIPTION: &'static str = "Check annotated variable types on runtime assignments.";
    const STATUS: Status = Status::OptIn;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_is_opt_in_and_can_be_parsed_and_disabled() {
        assert!(!ENFORCE_RUNTIME_ANNOTATIONS.get());
        assert!(
            crate::parse_iter(std::iter::once((
                "enforce-runtime-annotations".into(),
                None,
                ()
            )))
            .is_empty()
        );
        assert!(ENFORCE_RUNTIME_ANNOTATIONS.get());
        assert!(
            crate::parse_iter(std::iter::once((
                "enforce-typed-variables".into(),
                Some("false".into()),
                ()
            )))
            .is_empty()
        );
        assert!(!ENFORCE_RUNTIME_ANNOTATIONS.get());
    }
}
