use crate::{Span, Type, Value};

#[derive(Clone, Debug)]
pub struct Variable {
    pub declaration_span: Span,
    pub ty: Type,
    /// Whether the type was explicitly annotated, rather than inferred.
    pub type_annotated: bool,
    pub mutable: bool,
    pub const_val: Option<Value>,
}

impl Variable {
    pub fn new(declaration_span: Span, ty: Type, mutable: bool) -> Variable {
        Self {
            declaration_span,
            ty,
            type_annotated: false,
            mutable,
            const_val: None,
        }
    }
    /// Check explicitly annotated storage at runtime. Inferred types remain advisory.
    pub fn check_value(&self, value: &Value) -> Result<(), crate::ShellError> {
        if nu_experimental::ENFORCE_TYPED_VARIABLES.get()
            && self.type_annotated
            && !value.is_subtype_of(&self.ty)
        {
            return Err(crate::ShellError::CantConvert {
                to_type: self.ty.to_string(),
                from_type: value.get_type().to_string(),
                span: mismatch_span(value, &self.ty),
                help: Some("Value does not match the variable's type annotation".into()),
            });
        }
        Ok(())
    }
}

// For compound values, point to the offending element rather than the original
// container literal, which may have been initialized long before a reassignment.
fn mismatch_span(value: &Value, ty: &Type) -> Span {
    match (value, ty) {
        (Value::Record { val, .. }, Type::Record(fields)) => {
            for (name, ty) in fields.iter() {
                if let Some(value) = val.get(name) {
                    if !value.is_subtype_of(ty) {
                        return mismatch_span(value, ty);
                    }
                }
            }
        }
        (Value::List { vals, .. }, Type::List(inner)) => {
            if let Some(value) = vals.iter().find(|value| !value.is_subtype_of(inner)) {
                return mismatch_span(value, inner);
            }
        }
        (Value::List { vals, .. }, Type::Table(fields)) => {
            let row_type = Type::Record(fields.clone());
            if let Some(value) = vals.iter().find(|value| !value.is_subtype_of(&row_type)) {
                return mismatch_span(value, &row_type);
            }
        }
        _ => {}
    }
    value.span()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compound_mismatch_points_to_the_bad_value() {
        let bad_span = Span::new(30, 37);
        let value = Value::list(
            vec![
                Value::int(1, Span::new(10, 11)),
                Value::string("bad", bad_span),
            ],
            Span::new(1, 40),
        );
        assert_eq!(
            mismatch_span(&value, &Type::List(Box::new(Type::Int))),
            bad_span
        );
        let value = Value::record(crate::record! {"a" => value}, Span::new(0, 50));
        let ty = Type::Record(vec![("a".into(), Type::List(Box::new(Type::Int)))].into());
        assert_eq!(mismatch_span(&value, &ty), bad_span);
    }
}
