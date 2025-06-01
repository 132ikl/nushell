use super::trim_cstyle_null;
use chrono::{DateTime, FixedOffset, Local};
use nu_engine::command_prelude::*;
use sysinfo::System;

#[derive(Clone)]
pub struct SysHostname;

impl Command for SysHostname {
    fn name(&self) -> &str {
        "sys hostname"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys hostname")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn description(&self) -> &str {
        "View the hostname."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(host(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show hostname",
            example: "sys host",
            result: None,
        }]
    }
}

fn host(span: Span) -> Value {
    if let Some(hostname) = System::host_name() {
        Value::string(trim_cstyle_null(hostname), span)
    } else {
        Value::nothing(span)
    }
}
