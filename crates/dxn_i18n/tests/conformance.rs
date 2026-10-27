//! The Unicode MessageFormat Working Group's conformance tests (`tests/conformance/`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use std::sync::Arc;

use dxn_i18n::syntax::DataModelError;
use dxn_i18n::{
    Args, BidiIsolation, FormatError, FunctionContext, FunctionError, FunctionRegistry, Message,
    MessageFormatter, Options, ParseError, ResolvedValue, Value, downcast,
};
use dxn_intl::{DateTime, Number, parse_locale};
use serde_json::{Map, Value as Json};

/// `:test:function`, `:test:select` and `:test:format` from the suite's `test/README.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TestKind {
    Function,
    Select,
    Format,
}

#[derive(Debug)]
struct TestValue {
    kind: TestKind,
    input: f64,
    decimal_places: u8,
    fails_format: bool,
    fails_select: bool,
}

impl ResolvedValue for TestValue {
    fn format(&self) -> Result<String, FunctionError> {
        if self.kind == TestKind::Select {
            return Err(FunctionError::Failed("not-formattable".into()));
        }
        if self.fails_format {
            return Err(FunctionError::bad_option("fails", "a value that formats"));
        }
        let mut text = String::new();
        if self.input < 0.0 {
            text.push('-');
        }
        let abs = self.input.abs();
        text.push_str(&(abs.floor() as i64).to_string());
        if self.decimal_places == 1 {
            text.push('.');
            text.push_str(&(((abs - abs.floor()) * 10.0).floor() as i64).to_string());
        }
        Ok(text)
    }

    fn select(&self, keys: &[&str]) -> Option<Vec<usize>> {
        if self.kind == TestKind::Format || self.fails_select {
            return None;
        }
        let matches = |key: &str| match (self.input == 1.0, self.decimal_places) {
            (true, 1) => key == "1.0" || key == "1",
            (true, _) => key == "1",
            _ => false,
        };
        let mut order: Vec<usize> = (0..keys.len())
            .filter(|index| matches(keys[*index]))
            .collect();
        order.sort_by_key(|index| keys[*index] != "1.0");
        Some(order)
    }
}

fn test_function(kind: TestKind) -> impl dxn_i18n::Function {
    move |context: &mut FunctionContext<'_>,
          operand: Option<&Value>,
          options: &Options|
          -> Result<Arc<dyn ResolvedValue>, FunctionError> {
        let mut value = TestValue {
            kind,
            input: 0.0,
            decimal_places: 0,
            fails_format: false,
            fails_select: false,
        };
        match operand {
            Some(Value::Resolved(resolved)) if downcast::<TestValue>(resolved).is_some() => {
                let previous = downcast::<TestValue>(resolved).unwrap();
                value.input = previous.input;
                value.decimal_places = previous.decimal_places;
                value.fails_format = previous.fails_format;
                value.fails_select = previous.fails_select;
            }
            Some(Value::Number(number)) => value.input = number.to_f64(),
            Some(other) => {
                let text = match other {
                    Value::String(text) => text.clone(),
                    Value::Resolved(resolved) => resolved.format()?,
                    _ => return Err(FunctionError::BadOperand("not a number".into())),
                };
                let valid = !text.starts_with(['+', '.'])
                    && !text.ends_with('.')
                    && !text.starts_with("00");
                value.input = text
                    .parse::<f64>()
                    .ok()
                    .filter(|_| valid && text.parse::<Number>().is_ok())
                    .ok_or_else(|| FunctionError::BadOperand("not a number".into()))?;
            }
            None => return Err(FunctionError::BadOperand("no operand".into())),
        }
        if let Some(places) = options.text("decimalPlaces") {
            match places.as_str() {
                "0" => value.decimal_places = 0,
                "1" => value.decimal_places = 1,
                _ => return Err(FunctionError::bad_option("decimalPlaces", "0 or 1")),
            }
        }
        if let Some(fails) = options.text("fails") {
            match fails.as_str() {
                "always" => (value.fails_format, value.fails_select) = (true, true),
                "format" => value.fails_format = true,
                "select" => value.fails_select = true,
                "never" => {}
                _ => context.report(FunctionError::bad_option(
                    "fails",
                    "never, select, format or always",
                )),
            }
        }
        Ok(Arc::new(value))
    }
}

fn functions() -> Arc<FunctionRegistry> {
    let mut functions = FunctionRegistry::default();
    functions
        .register("test:function", test_function(TestKind::Function))
        .register("test:select", test_function(TestKind::Select))
        .register("test:format", test_function(TestKind::Format));
    Arc::new(functions)
}

fn error_type(error: &FormatError) -> &'static str {
    match error {
        FormatError::UnresolvedVariable(_) => "unresolved-variable",
        FormatError::UnknownFunction(_) => "unknown-function",
        FormatError::BadOperand { .. } => "bad-operand",
        FormatError::BadOption { .. } => "bad-option",
        FormatError::BadSelector(_) => "bad-selector",
        FormatError::Function { reason, .. } if reason == "not-formattable" => "not-formattable",
        FormatError::Function { .. } => "message-function",
    }
}

fn parse_error_type(error: &ParseError) -> &'static str {
    match error {
        ParseError::Syntax { .. } => "syntax-error",
        ParseError::DataModel(DataModelError::VariantKeyMismatch) => "variant-key-mismatch",
        ParseError::DataModel(DataModelError::MissingFallbackVariant) => "missing-fallback-variant",
        ParseError::DataModel(DataModelError::MissingSelectorAnnotation(_)) => {
            "missing-selector-annotation"
        }
        ParseError::DataModel(DataModelError::DuplicateDeclaration(_)) => "duplicate-declaration",
        ParseError::DataModel(DataModelError::DuplicateOptionName(_)) => "duplicate-option-name",
        ParseError::DataModel(DataModelError::DuplicateVariant) => "duplicate-variant",
    }
}

fn param(param: &Json) -> (String, Value) {
    let name = param["name"].as_str().unwrap().to_string();
    let value = match (&param["value"], param.get("type").and_then(Json::as_str)) {
        (Json::String(text), Some("datetime")) => {
            let (date, time) = text.split_once('T').unwrap();
            let date: Vec<i32> = date.split('-').map(|part| part.parse().unwrap()).collect();
            let time: Vec<u8> = time.split(':').map(|part| part.parse().unwrap()).collect();
            Value::DateTime(
                DateTime::new(
                    date[0],
                    date[1] as u8,
                    date[2] as u8,
                    time[0],
                    time[1],
                    time[2],
                )
                .unwrap(),
            )
        }
        (Json::String(text), _) => Value::String(text.clone()),
        (Json::Number(number), _) if number.is_i64() => Value::from(number.as_i64().unwrap()),
        (Json::Number(number), _) => Value::from(number.as_f64().unwrap()),
        (Json::Bool(flag), _) => Value::String(flag.to_string()),
        (other, _) => panic!("unsupported parameter {other}"),
    };
    (name, value)
}

/// Runs one file's tests; returns a description of each failure.
fn run_file(path: &Path, functions: &Arc<FunctionRegistry>) -> Vec<String> {
    let suite: Json = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let defaults = suite
        .get("defaultTestProperties")
        .and_then(Json::as_object)
        .cloned()
        .unwrap_or_default();
    let mut failures = Vec::new();
    for test in suite["tests"].as_array().unwrap() {
        let mut properties: Map<String, Json> = defaults.clone();
        properties.extend(test.as_object().unwrap().clone());
        let source = properties["src"].as_str().unwrap();
        let locale = parse_locale(
            properties
                .get("locale")
                .and_then(Json::as_str)
                .unwrap_or("en-US"),
        )
        .unwrap();
        let bidi = match properties.get("bidiIsolation").and_then(Json::as_str) {
            Some("default") => BidiIsolation::Default,
            _ => BidiIsolation::None,
        };
        let args: Args = properties
            .get("params")
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
            .map(param)
            .collect();
        let (text, mut errors): (Option<String>, Vec<&str>) = match Message::parse(source) {
            Ok(message) => {
                let formatter = MessageFormatter::new(locale)
                    .with_functions(functions.clone())
                    .with_bidi_isolation(bidi);
                let formatted = formatter.format(&message, &args);
                (
                    Some(formatted.text),
                    formatted.errors.iter().map(error_type).collect(),
                )
            }
            Err(error) => (None, vec![parse_error_type(&error)]),
        };
        let mut problems = Vec::new();
        if let Some(expected) = properties.get("exp").and_then(Json::as_str)
            && text.as_deref() != Some(expected)
        {
            problems.push(format!("text {text:?}, expected {expected:?}"));
        }
        if let Some(expected) = properties.get("expErrors").and_then(Json::as_array) {
            let mut expected: Vec<&str> = expected
                .iter()
                .map(|error| error["type"].as_str().unwrap())
                .collect();
            expected.sort_unstable();
            errors.sort_unstable();
            if errors != expected {
                problems.push(format!("errors {errors:?}, expected {expected:?}"));
            }
        }
        if !problems.is_empty() {
            failures.push(format!(
                "{}: {source:?}: {}",
                path.display(),
                problems.join("; ")
            ));
        }
    }
    failures
}

#[test]
fn the_working_groups_suite_passes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance");
    let mut files: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .chain(std::fs::read_dir(root.join("functions")).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    files.sort();
    assert!(files.len() >= 16, "the suite's files are missing");
    let functions = functions();
    let failures: Vec<String> = files
        .iter()
        .flat_map(|file| run_file(file, &functions))
        .collect();
    assert!(
        failures.is_empty(),
        "{} conformance failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
