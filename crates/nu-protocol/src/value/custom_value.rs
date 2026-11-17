use std::{cmp::Ordering, fmt};

use crate::{ShellError, Span, Type, Value, ast::Operator};

/// Trait definition for a custom [`Value`](crate::Value) type
#[typetag::serde(tag = "type")]
pub trait CustomValue: fmt::Debug + Send + Sync {
    /// Custom `Clone` implementation
    ///
    /// This can reemit a `Value::Custom(Self, span)` or materialize another representation
    /// if necessary.
    fn clone_value(&self, span: Span) -> Value;

    //fn category(&self) -> Category;

    /// The friendly type name to show for the custom value, e.g. in `describe` and in error
    /// messages. This does not have to be the same as the name of the struct or enum, but
    /// conventionally often is.
    fn type_name(&self) -> String;

    /// Converts the custom value to a base nushell value.
    ///
    /// This imposes the requirement that you can represent the custom value in some form using the
    /// Value representations that already exist in nushell
    fn to_base_value(&self, span: Span) -> Result<Value, ShellError>;

    /// Any representation used to downcast object to its original type
    fn as_any(&self) -> &dyn std::any::Any;

    /// Any representation used to downcast object to its original type (mutable reference)
    fn as_mut_any(&mut self) -> &mut dyn std::any::Any;

    /// Follow cell path by numeric index (e.g. rows)
    fn follow_path_int(
        &self,
        self_span: Span,
        index: usize,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        let _ = (self_span, index);
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.type_name(),
            span: path_span,
        })
    }

    /// Follow cell path by string key (e.g. columns)
    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        let _ = (self_span, column_name);
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.type_name(),
            span: path_span,
        })
    }

    /// Follow an index while preserving the cell-path optional flag.
    /// Existing implementations may continue to implement `follow_path_int`.
    fn follow_path_int_with_options(
        &self,
        self_span: Span,
        index: usize,
        path_span: Span,
        _optional: bool,
    ) -> Result<Value, ShellError> {
        self.follow_path_int(self_span, index, path_span)
    }

    /// Follow a key with the cell-path optional flag and casing policy.
    fn follow_path_string_with_options(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
        _optional: bool,
        _casing: crate::casing::Casing,
    ) -> Result<Value, ShellError> {
        self.follow_path_string(self_span, column_name, path_span)
    }

    /// ordering with other value (see [`std::cmp::PartialOrd`])
    fn partial_cmp(&self, _other: &Value) -> Option<Ordering> {
        None
    }

    /// Definition of an operation between the object that implements the trait
    /// and another Value.
    ///
    /// The Operator enum is used to indicate the expected operation.
    ///
    /// Default impl raises [`ShellError::OperatorUnsupportedType`].
    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        let _ = (lhs_span, right);
        Err(ShellError::OperatorUnsupportedType {
            op: operator,
            unsupported: Type::Custom(self.type_name().into()),
            op_span: op,
            unsupported_span: lhs_span,
            help: None,
        })
    }

    /// For custom values in plugins: return `true` here if you would like to be notified when all
    /// copies of this custom value are dropped in the engine.
    ///
    /// The notification will take place via `custom_value_dropped()` on the plugin type.
    ///
    /// The default is `false`.
    fn notify_plugin_on_drop(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod predictable_options_tests {
    use super::*;
    use crate::{ast::PathMember, casing::Casing};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct OptionsProbe;
    #[typetag::serde]
    impl CustomValue for OptionsProbe {
        fn clone_value(&self, span: Span) -> Value {
            Value::custom(Box::new(self.clone()), span)
        }
        fn type_name(&self) -> String {
            "options probe".into()
        }
        fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
            Ok(Value::nothing(span))
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
            self
        }
        fn follow_path_string_with_options(
            &self,
            _: Span,
            _: String,
            span: Span,
            optional: bool,
            casing: Casing,
        ) -> Result<Value, ShellError> {
            Ok(Value::bool(optional && casing == Casing::Insensitive, span))
        }
        fn follow_path_int_with_options(
            &self,
            _: Span,
            _: usize,
            span: Span,
            optional: bool,
        ) -> Result<Value, ShellError> {
            Ok(Value::bool(optional, span))
        }
    }
    #[test]
    fn predictable_custom_cell_path_options() {
        let value = Value::test_custom_value(Box::new(OptionsProbe));
        assert_eq!(
            value
                .follow_cell_path(&[PathMember::test_string(
                    "key".into(),
                    true,
                    Casing::Insensitive
                )])
                .unwrap()
                .as_ref(),
            &Value::test_bool(true)
        );
        assert_eq!(
            value
                .follow_cell_path(&[PathMember::test_int(0, true)])
                .unwrap()
                .as_ref(),
            &Value::test_bool(true)
        );
    }
}
