use nu_engine::command_prelude::*;
use nu_protocol::{
    style::{self, StyleOptions, StyleSpan},
    Range::{FloatRange, IntRange},
};

#[derive(Clone)]
pub struct Style;

impl Command for Style {
    fn name(&self) -> &str {
        "style"
    }

    fn signature(&self) -> Signature {
        Signature::build("style")
            .input_output_types(vec![(Type::String, Type::String)])
            .optional(
                "span",
                SyntaxShape::Range,
                "Span within string to apply style to.",
            )
            .category(Category::Strings)
            .allow_variants_without_examples(true)
    }

    fn description(&self) -> &str {
        "Add a style to a string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["style", "color"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }

    fn extra_description(&self) -> &str {
        ""
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let range = call.opt(engine_state, stack, 0)?;
        let metadata = input.metadata();

        let (string, mut styles) = match input {
            PipelineData::Value(Value::String { val, styles, .. }, ..) => (val, styles),
            _ => todo!(),
        };

        let style_span = match range {
            Some(Value::Range { val, .. }) => match *val {
                IntRange(range) => {
                    // TODO: check step
                    let end = match range.end() {
                        std::ops::Bound::Included(x) => x + 1,
                        std::ops::Bound::Excluded(x) => x,
                        std::ops::Bound::Unbounded => todo!(),
                    };
                    StyleSpan::new(range.start() as usize, end as usize)
                }
                FloatRange(_) => todo!(),
            },
            Some(val) => {
                return Err(ShellError::TypeMismatch {
                    err_message: "Argument must be an integer range".into(),
                    span: val.span(),
                })
            }
            None => StyleSpan::new(0, string.len()),
        };

        let options = style::StyleOptions::Color(255, 0, 0);
        let style = style::Style::new(options, style_span);

        styles.push(style);

        Ok(Value::styled_string(string, styles, head).into_pipeline_data_with_metadata(metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Style {})
    }
}
