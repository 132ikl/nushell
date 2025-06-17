use std::{borrow::Cow, sync::Arc};

use fancy_regex::{Captures, Regex};

use crate::{
    IntoPipelineData, Span, Spanned, Value,
    ast::Call,
    engine::{EngineState, Stack},
    record,
};

/// ANSI style reset
const RESET: &str = "\x1b[0m";
/// ANSI set default dimmed
const DEFAULT_DIMMED: &str = "\x1b[2;39m";
/// ANSI set default italic
const DEFAULT_ITALIC: &str = "\x1b[3;39m";

/// Syntax highlight code using the `nu-highlight` command if available
pub fn try_nu_highlight(
    code_string: &str,
    reject_garbage: bool,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Option<String> {
    let highlighter = engine_state.find_decl(b"nu-highlight", &[])?;

    let decl = engine_state.get_decl(highlighter);
    let mut call = Call::new(Span::unknown());
    if reject_garbage {
        call.add_named((
            Spanned {
                item: "reject-garbage".into(),
                span: Span::unknown(),
            },
            None,
            None,
        ));
    }

    decl.run(
        engine_state,
        stack,
        &(&call).into(),
        Value::string(code_string, Span::unknown()).into_pipeline_data(),
    )
    .and_then(|pipe| pipe.into_value(Span::unknown()))
    .and_then(|val| val.coerce_into_string())
    .ok()
}

/// Syntax highlight code using the `nu-highlight` command if available, falling back to the given string
pub fn nu_highlight_string(
    code_string: &str,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> String {
    try_nu_highlight(code_string, false, engine_state, stack)
        .unwrap_or_else(|| code_string.to_string())
}

/// Highlight code within backticks
///
/// Will attempt to use nu-highlight, falling back to dimmed and italic on invalid syntax
pub fn highlight_code<'a>(
    text: &'a str,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Cow<'a, str> {
    let config = stack.get_config(engine_state);
    if !config.use_ansi_coloring.get(engine_state) {
        return Cow::Borrowed(text);
    }

    // See [`tests::test_code_formatting`] for examples
    let pattern = r"(?x)     # verbose mode
        (?<![\p{Letter}\d])    # negative look-behind for alphanumeric: ensure backticks are not directly preceded by letter/number.
        `
        ([^`\n]+?)           # capture characters inside backticks, excluding backticks and newlines. ungreedy.
        `
        (?![\p{Letter}\d])     # negative look-ahead for alphanumeric: ensure backticks are not directly followed by letter/number.
    ";

    let re = Regex::new(pattern).expect("regex failed to compile");
    let do_try_highlight =
        |captures: &Captures| highlight_capture_group(captures, engine_state, stack);
    re.replace_all(text, do_try_highlight)
}

/// Apply code highlighting to code in a capture group
fn highlight_capture_group(
    captures: &Captures,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> String {
    let Some(content) = captures.get(1) else {
        // this shouldn't happen
        return String::new();
    };

    // Save current color config
    let config_old = stack.get_config(engine_state);
    let mut config = (*config_old).clone();

    // Style externals and external arguments with fallback style,
    // so nu-highlight styles code which is technically valid syntax,
    // but not an internal command is highlighted with the fallback style
    let code_style = Value::record(
        record! {
            "attr" => Value::string("di", Span::unknown()),
        },
        Span::unknown(),
    );
    let color_config = &mut config.color_config;
    color_config.insert("shape_external".into(), code_style.clone());
    color_config.insert("shape_external_resolved".into(), code_style.clone());
    color_config.insert("shape_externalarg".into(), code_style);

    // Apply config with external argument style
    stack.config = Some(Arc::new(config));

    // Highlight and reject invalid syntax
    let highlighted = try_nu_highlight(content.into(), true, engine_state, stack);

    // Restore original config
    stack.config = Some(config_old);

    // Use fallback style if highlight failed/syntax was invalid
    highlighted.unwrap_or_else(|| highlight_fallback(content.into()))
}

/// Apply fallback code style
fn highlight_fallback(text: &str) -> String {
    format!("{DEFAULT_DIMMED}{DEFAULT_ITALIC}{text}{RESET}")
}
