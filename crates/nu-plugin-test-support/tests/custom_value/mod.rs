use std::cmp::Ordering;

use nu_plugin::{EngineInterface, EvaluatedCall, Plugin, SimplePluginCommand};
use nu_plugin_test_support::PluginTest;
use nu_protocol::{
    CustomValue, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
struct CustomU32(u32);

impl CustomU32 {
    pub fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }
}

#[typetag::serde]
impl CustomValue for CustomU32 {
    fn clone_value(&self, span: Span) -> Value {
        self.clone().into_value(span)
    }

    fn type_name(&self) -> String {
        "CustomU32".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::int(self.0 as i64, span))
    }

    fn save(
        &self,
        path: nu_protocol::Spanned<&std::path::Path>,
        _: Span,
        save_span: Span,
    ) -> Result<(), ShellError> {
        std::fs::write(path.item, format!("plugin u32: {}", self.0)).map_err(|error| {
            ShellError::Io(nu_protocol::shell_error::io::IoError::new(
                error,
                save_span,
                Some(path.item.into()),
            ))
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        other
            .as_custom_value()
            .ok()
            .and_then(|cv| cv.as_any().downcast_ref::<CustomU32>())
            .and_then(|other_u32| PartialOrd::partial_cmp(self, other_u32))
    }
}

struct CustomU32Plugin;
struct IntoU32;
struct IntoIntFromU32;

impl Plugin for CustomU32Plugin {
    fn version(&self) -> String {
        "0.0.0".into()
    }

    fn commands(&self) -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = Self>>> {
        vec![Box::new(IntoU32), Box::new(IntoIntFromU32)]
    }
}

impl SimplePluginCommand for IntoU32 {
    type Plugin = CustomU32Plugin;

    fn name(&self) -> &str {
        "into u32"
    }

    fn description(&self) -> &str {
        "Convert a number to a 32-bit unsigned integer"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).input_output_type(Type::Int, Type::Custom("CustomU32".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "340 | into u32",
            description: "Make a u32",
            result: Some(CustomU32(340).into_value(Span::test_data())),
        }]
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let value: i64 = input.as_int()?;
        let value_u32 = u32::try_from(value).map_err(|err| {
            LabeledError::new(format!("Not a valid u32: {value}"))
                .with_label(err.to_string(), input.span())
        })?;
        Ok(CustomU32(value_u32).into_value(call.head))
    }
}

impl SimplePluginCommand for IntoIntFromU32 {
    type Plugin = CustomU32Plugin;

    fn name(&self) -> &str {
        "into int from u32"
    }

    fn description(&self) -> &str {
        "Turn a u32 back into a number"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).input_output_type(Type::Custom("CustomU32".into()), Type::Int)
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let value: &CustomU32 = input
            .as_custom_value()?
            .as_any()
            .downcast_ref()
            .ok_or_else(|| ShellError::TypeMismatch {
                err_message: "expected CustomU32".into(),
                span: input.span(),
            })?;
        Ok(Value::int(value.0 as i64, call.head))
    }
}

#[test]
fn test_into_u32_examples() -> Result<(), ShellError> {
    PluginTest::new("custom_u32", CustomU32Plugin.into())?.test_command_examples(&IntoU32)
}

#[test]
fn test_into_int_from_u32() -> Result<(), ShellError> {
    let result = PluginTest::new("custom_u32", CustomU32Plugin.into())?
        .eval_with(
            "into int from u32",
            PipelineData::value(CustomU32(42).into_value(Span::test_data()), None),
        )?
        .into_value(Span::test_data())?;
    assert_eq!(Value::test_int(42), result);
    Ok(())
}

#[derive(Clone)]
struct SaveCustom;
impl nu_protocol::engine::Command for SaveCustom {
    fn name(&self) -> &str {
        "save-custom"
    }
    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("path", nu_protocol::SyntaxShape::Filepath, "Destination.")
            .input_output_type(Type::Any, Type::Nothing)
    }
    fn description(&self) -> &str {
        "Exercise host dispatch of a plugin value's save method."
    }
    fn run(
        &self,
        engine: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &nu_protocol::engine::Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        use nu_engine::CallExt;
        let path: nu_protocol::Spanned<std::path::PathBuf> = call.req(engine, stack, 0)?;
        let value = input.into_value(call.head)?;
        value.as_custom_value()?.save(
            path.as_ref().map(|path| path.as_path()),
            value.span(),
            call.head,
        )?;
        Ok(PipelineData::empty())
    }
}

#[test]
fn plugin_custom_save_reaches_the_originating_value() -> Result<(), Box<ShellError>> {
    let path = std::env::temp_dir().join(format!(
        "nu-plugin-save-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut plugin = PluginTest::new("custom_u32", CustomU32Plugin.into())?;
    plugin.add_decl(Box::new(SaveCustom))?;
    plugin.eval(&format!("42 | into u32 | save-custom '{}'", path.display()))?;
    let contents = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(contents, "plugin u32: 42");
    let error = plugin
        .eval_with(
            &format!("save-custom '{}'", path.display()),
            PipelineData::value(
                Value::test_custom_value(Box::new(nu_plugin_protocol::test_util::TestCustomValue(
                    1,
                ))),
                None,
            ),
        )
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Custom value does not implement `save`"),
        "{error:?}"
    );
    assert!(!path.exists());
    Ok(())
}
