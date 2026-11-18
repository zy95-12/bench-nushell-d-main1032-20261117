use super::{config_update_string_enum, prelude::*};
use crate as nu_protocol;
use std::{num::NonZeroU16, time::Duration};

#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableMode {
    Basic,
    Thin,
    Light,
    Compact,
    WithLove,
    CompactDouble,
    #[default]
    Rounded,
    Reinforced,
    Heavy,
    None,
    Psql,
    Markdown,
    Dots,
    Restructured,
    AsciiRounded,
    BasicCompact,
    Single,
    Double,
}

impl FromStr for TableMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "basic" => Ok(Self::Basic),
            "thin" => Ok(Self::Thin),
            "light" => Ok(Self::Light),
            "compact" => Ok(Self::Compact),
            "with_love" => Ok(Self::WithLove),
            "compact_double" => Ok(Self::CompactDouble),
            "default" => Ok(TableMode::default()),
            "rounded" => Ok(Self::Rounded),
            "reinforced" => Ok(Self::Reinforced),
            "heavy" => Ok(Self::Heavy),
            "none" => Ok(Self::None),
            "psql" => Ok(Self::Psql),
            "markdown" => Ok(Self::Markdown),
            "dots" => Ok(Self::Dots),
            "restructured" => Ok(Self::Restructured),
            "ascii_rounded" => Ok(Self::AsciiRounded),
            "basic_compact" => Ok(Self::BasicCompact),
            "single" => Ok(Self::Single),
            "double" => Ok(Self::Double),
            _ => Err(
                "'basic', 'thin', 'light', 'compact', 'with_love', 'compact_double', 'rounded', 'reinforced', 'heavy', 'none', 'psql', 'markdown', 'dots', 'restructured', 'ascii_rounded', 'basic_compact', 'single', or 'double'",
            ),
        }
    }
}

impl UpdateFromValue for TableMode {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FooterMode {
    /// Never show the footer
    Never,
    /// Always show the footer
    Always,
    /// Only show the footer if there are more than RowCount rows
    RowCount(u64),
    /// Calculate the screen height and row count, if screen height is larger than row count, don't show footer
    Auto,
}

impl FromStr for FooterMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "always" => Ok(FooterMode::Always),
            "never" => Ok(FooterMode::Never),
            "auto" => Ok(FooterMode::Auto),
            _ => Err("'never', 'always', 'auto', or int"),
        }
    }
}

impl UpdateFromValue for FooterMode {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        match value {
            Value::String { val, .. } => match val.parse() {
                Ok(val) => *self = val,
                Err(err) => errors.invalid_value(path, err.to_string(), value),
            },
            &Value::Int { val, .. } => {
                if val >= 0 {
                    *self = Self::RowCount(val as u64);
                } else {
                    errors.invalid_value(path, "a non-negative integer", value);
                }
            }
            _ => errors.type_mismatch(
                path,
                Type::custom("'never', 'always', 'auto', or int"),
                value,
            ),
        }
    }
}

impl IntoValue for FooterMode {
    fn into_value(self, span: Span) -> Value {
        match self {
            FooterMode::Always => "always".into_value(span),
            FooterMode::Never => "never".into_value(span),
            FooterMode::Auto => "auto".into_value(span),
            FooterMode::RowCount(c) => (c as i64).into_value(span),
        }
    }
}

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableIndexMode {
    /// Always show indexes
    Always,
    /// Never show indexes
    Never,
    /// Show indexes when a table has "index" column
    Auto,
}

impl FromStr for TableIndexMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "always" => Ok(TableIndexMode::Always),
            "never" => Ok(TableIndexMode::Never),
            "auto" => Ok(TableIndexMode::Auto),
            _ => Err("'never', 'always' or 'auto'"),
        }
    }
}

impl UpdateFromValue for TableIndexMode {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors)
    }
}

/// A Table view configuration, for a situation where
/// we need to limit cell width in order to adjust for a terminal size.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrimStrategy {
    /// Wrapping strategy.
    ///
    /// It it's similar to original nu_table, strategy.
    Wrap {
        /// A flag which indicates whether is it necessary to try
        /// to keep word boundaries.
        try_to_keep_words: bool,
    },
    /// Truncating strategy, where we just cut the string.
    /// And append the suffix if applicable.
    Truncate {
        /// Suffix which can be appended to a truncated string after being cut.
        ///
        /// It will be applied only when there's enough room for it.
        /// For example in case where a cell width must be 12 chars, but
        /// the suffix takes 13 chars it won't be used.
        suffix: Option<String>,
    },
}

impl TrimStrategy {
    pub fn wrap(dont_split_words: bool) -> Self {
        Self::Wrap {
            try_to_keep_words: dont_split_words,
        }
    }

    pub fn truncate(suffix: Option<String>) -> Self {
        Self::Truncate { suffix }
    }
}

impl Default for TrimStrategy {
    fn default() -> Self {
        Self::Wrap {
            try_to_keep_words: true,
        }
    }
}

impl IntoValue for TrimStrategy {
    fn into_value(self, span: Span) -> Value {
        match self {
            TrimStrategy::Wrap { try_to_keep_words } => {
                record! {
                    "methodology" => "wrapping".into_value(span),
                    "wrapping_try_keep_words" => try_to_keep_words.into_value(span),
                }
            }
            TrimStrategy::Truncate { suffix } => {
                record! {
                    "methodology" => "truncating".into_value(span),
                    "truncating_suffix" => suffix.into_value(span),
                }
            }
        }
        .into_value(span)
    }
}

impl UpdateFromValue for TrimStrategy {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        let Some(methodology) = record.get("methodology") else {
            errors.missing_column(path, "methodology", value.span());
            return;
        };

        match methodology.as_str() {
            Ok("wrapping") => {
                let mut try_to_keep_words = if let &mut Self::Wrap { try_to_keep_words } = self {
                    try_to_keep_words
                } else {
                    false
                };
                for (col, val) in record.iter() {
                    let path = &mut path.push(col);
                    match col.as_str() {
                        "wrapping_try_keep_words" => try_to_keep_words.update(val, path, errors),
                        "methodology" | "truncating_suffix" => (),
                        _ => errors.unknown_option(path, val),
                    }
                }
                *self = Self::Wrap { try_to_keep_words };
            }
            Ok("truncating") => {
                let mut suffix = if let Self::Truncate { suffix } = self {
                    suffix.take()
                } else {
                    None
                };
                for (col, val) in record.iter() {
                    let path = &mut path.push(col);
                    match col.as_str() {
                        "truncating_suffix" => match val {
                            Value::Nothing { .. } => suffix = None,
                            Value::String { val, .. } => suffix = Some(val.clone()),
                            _ => errors.type_mismatch(path, Type::String, val),
                        },
                        "methodology" | "wrapping_try_keep_words" => (),
                        _ => errors.unknown_option(path, val),
                    }
                }
                *self = Self::Truncate { suffix };
            }
            Ok(_) => errors.invalid_value(
                &path.push("methodology"),
                "'wrapping' or 'truncating'",
                methodology,
            ),
            Err(_) => errors.type_mismatch(&path.push("methodology"), Type::String, methodology),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableIndent {
    pub left: usize,
    pub right: usize,
}

impl TableIndent {
    pub fn new(left: usize, right: usize) -> Self {
        Self { left, right }
    }
}

impl IntoValue for TableIndent {
    fn into_value(self, span: Span) -> Value {
        record! {
            "left" => (self.left as i64).into_value(span),
            "right" => (self.right as i64).into_value(span),
        }
        .into_value(span)
    }
}

impl Default for TableIndent {
    fn default() -> Self {
        Self { left: 1, right: 1 }
    }
}

impl UpdateFromValue for TableIndent {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        match value {
            &Value::Int { val, .. } => {
                if let Ok(val) = val.try_into() {
                    self.left = val;
                    self.right = val;
                } else {
                    errors.invalid_value(path, "a non-negative integer", value);
                }
            }
            Value::Record { val: record, .. } => {
                for (col, val) in record.iter() {
                    let path = &mut path.push(col);
                    match col.as_str() {
                        "left" => self.left.update(val, path, errors),
                        "right" => self.right.update(val, path, errors),
                        _ => errors.unknown_option(path, val),
                    }
                }
            }
            _ => errors.type_mismatch(path, Type::custom("int or record"), value),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableConfig {
    pub mode: TableMode,
    /// Maximum buffering duration per streamed table page.
    pub batch_duration: Duration,
    /// Maximum rows per streamed table page.
    pub stream_page_size: NonZeroU16,
    /// Compatibility mirror of `stream_page_size`; prefer the canonical field.
    pub page_size: usize,
    pub index_mode: TableIndexMode,
    pub show_empty: bool,
    pub padding: TableIndent,
    pub trim: TrimStrategy,
    pub header_on_separator: bool,
    pub abbreviated_row_count: Option<usize>,
    pub footer_inheritance: bool,
    pub missing_value_symbol: String,
}

impl IntoValue for TableConfig {
    fn into_value(self, span: Span) -> Value {
        let abbv_count = self
            .abbreviated_row_count
            .map(|t| t as i64)
            .into_value(span);

        record! {
            "mode" => self.mode.into_value(span),
            "batch_duration" => Value::duration(self.batch_duration.as_nanos().min(i64::MAX as u128) as i64, span),
            "page_size" => (self.page_size as i64).into_value(span),
            "stream_page_size" => i64::from(self.stream_page_size.get()).into_value(span),
            "index_mode" => self.index_mode.into_value(span),
            "show_empty" => self.show_empty.into_value(span),
            "padding" => self.padding.into_value(span),
            "trim" => self.trim.into_value(span),
            "header_on_separator" => self.header_on_separator.into_value(span),
            "abbreviated_row_count" => abbv_count,
            "footer_inheritance" => self.footer_inheritance.into_value(span),
            "missing_value_symbol" => self.missing_value_symbol.into_value(span),
        }
        .into_value(span)
    }
}

impl Default for TableConfig {
    fn default() -> Self {
        Self {
            mode: TableMode::Rounded,
            batch_duration: Duration::from_secs(1),
            stream_page_size: NonZeroU16::new(1000).expect("positive default"),
            page_size: 1000,
            index_mode: TableIndexMode::Always,
            show_empty: true,
            trim: TrimStrategy::default(),
            header_on_separator: false,
            padding: TableIndent::default(),
            abbreviated_row_count: None,
            footer_inheritance: false,
            missing_value_symbol: "❎".into(),
        }
    }
}

impl UpdateFromValue for TableConfig {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        // A changed canonical setting wins. When a full config record only
        // changes the legacy mirror, honor that change instead. Resolve this
        // before iterating so record insertion order cannot change the result.
        let canonical = record.get("stream_page_size");
        let legacy = record.get("page_size");
        let selected_page_size = if canonical.is_some_and(|value| {
            value.as_int().ok() == Some(i64::from(self.stream_page_size.get()))
        }) && legacy
            .is_some_and(|value| value.as_int().ok() != Some(self.page_size as i64))
        {
            "page_size"
        } else if canonical.is_some() {
            "stream_page_size"
        } else {
            "page_size"
        };
        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "mode" => self.mode.update(val, path, errors),
                "batch_duration" => match val.as_duration() {
                    Ok(duration) if duration >= 0 => {
                        self.batch_duration = Duration::from_nanos(duration as u64)
                    }
                    Ok(_) => errors.invalid_value(path, "a non-negative duration", val),
                    Err(_) => errors.type_mismatch(path, Type::Duration, val),
                },
                "page_size" | "stream_page_size" => match val.as_int() {
                    Ok(size) if size > 0 && u16::try_from(size).is_ok() => {
                        if col == selected_page_size {
                            self.page_size = size as usize;
                            self.stream_page_size =
                                NonZeroU16::new(size as u16).expect("validated positive size");
                        }
                    }
                    Ok(_) => errors.invalid_value(path, "a positive integer", val),
                    Err(_) => errors.type_mismatch(path, Type::Int, val),
                },
                "index_mode" => self.index_mode.update(val, path, errors),
                "show_empty" => self.show_empty.update(val, path, errors),
                "trim" => self.trim.update(val, path, errors),
                "header_on_separator" => self.header_on_separator.update(val, path, errors),
                "padding" => self.padding.update(val, path, errors),
                "abbreviated_row_count" => match val {
                    Value::Nothing { .. } => self.abbreviated_row_count = None,
                    &Value::Int { val: count, .. } => {
                        if let Ok(count) = count.try_into() {
                            self.abbreviated_row_count = Some(count);
                        } else {
                            errors.invalid_value(path, "a non-negative integer", val);
                        }
                    }
                    _ => errors.type_mismatch(path, Type::custom("int or nothing"), val),
                },
                "footer_inheritance" => self.footer_inheritance.update(val, path, errors),
                "missing_value_symbol" => match val.as_str() {
                    Ok(val) => self.missing_value_symbol = val.to_string(),
                    Err(_) => errors.type_mismatch(path, Type::String, val),
                },
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

#[cfg(test)]
mod streaming_tests {
    use super::*;
    use crate::Config;

    #[test]
    fn streaming_table_settings_update_and_serialize() {
        let old = Config::default();
        let mut config = old.clone();
        let value = Value::test_record(record! {
            "table" => Value::test_record(record! {
                "batch_duration" => Value::test_duration(20_000_000),
                "page_size" => Value::test_int(2),
            }),
        });
        config.update_from_value(&old, &value).unwrap();
        let serialized = config.into_value(Span::test_data());
        let table = serialized
            .as_record()
            .unwrap()
            .get("table")
            .unwrap()
            .as_record()
            .unwrap();
        assert_eq!(
            table.get("batch_duration").unwrap(),
            &Value::test_duration(20_000_000)
        );
        assert_eq!(table.get("page_size").unwrap(), &Value::test_int(2));
    }

    #[test]
    fn streaming_table_settings_reject_invalid_limits() {
        for (key, value) in [
            ("page_size", Value::test_int(0)),
            ("page_size", Value::test_int(-1)),
            ("stream_page_size", Value::test_int(0)),
            ("stream_page_size", Value::test_int(65536)),
            ("batch_duration", Value::test_duration(-1)),
            ("batch_duration", Value::test_int(1)),
        ] {
            let old = Config::default();
            let mut config = old.clone();
            let update =
                Value::test_record(record! {"table" => Value::test_record(record! {key => value})});
            assert!(config.update_from_value(&old, &update).is_err());
            assert_eq!(
                config.into_value(Span::test_data()),
                old.into_value(Span::test_data())
            );
        }
    }
}

#[cfg(test)]
mod canonical_streaming_tests {
    use super::*;
    #[test]
    fn canonical_defaults_and_both_setting_names() {
        let table = TableConfig::default();
        assert_eq!(table.batch_duration, Duration::from_secs(1));
        assert_eq!(table.stream_page_size.get(), 1000);
        for key in ["page_size", "stream_page_size"] {
            let old = crate::Config::default();
            let mut table = old
                .table
                .clone()
                .into_value(Span::test_data())
                .as_record()
                .unwrap()
                .clone();
            table.insert(key, Value::test_int(3));
            let serialized = Value::test_record(record! { "table" => Value::test_record(table) });
            let mut config = old.clone();
            config.update_from_value(&old, &serialized).unwrap();
            assert_eq!(config.table.stream_page_size.get(), 3, "{key}");
            assert_eq!(config.table.page_size, 3, "{key}");
        }
    }
}

#[cfg(test)]
mod streaming_setting_order_tests {
    use super::*;
    #[test]
    fn canonical_and_legacy_settings_do_not_depend_on_record_order() {
        for (canonical, legacy, expected) in [(7, 3, 7), (1000, 3, 3), (7, 1000, 7)] {
            for reverse in [false, true] {
                let old = crate::Config::default();
                let mut fields = vec![("stream_page_size", canonical), ("page_size", legacy)];
                if reverse {
                    fields.reverse();
                }
                let table = fields
                    .into_iter()
                    .map(|(name, value)| (name.to_string(), Value::test_int(value)))
                    .collect::<crate::Record>();
                let mut config = old.clone();
                let update = Value::test_record(record! { "table" => Value::test_record(table) });
                config.update_from_value(&old, &update).unwrap();
                assert_eq!(config.table.page_size, expected, "reverse={reverse}");
                assert_eq!(usize::from(config.table.stream_page_size.get()), expected);
            }
        }
    }
}
