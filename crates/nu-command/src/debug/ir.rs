use crossterm::event::{Event, KeyCode, KeyEventKind};
use nu_engine::{command_prelude::*, ClosureEvalOnce};
use nu_protocol::{debugger::IrDebugger, engine::Closure, BlockId, DeclId};
use std::time::Duration;

#[derive(Clone)]
pub struct DebugIr;

impl Command for DebugIr {
    fn name(&self) -> &str {
        "debug ir"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "closure",
                SyntaxShape::Closure(None),
                "The closure to profile.",
            )
            .input_output_type(Type::Nothing, Type::String)
            .category(Category::Debug)
    }

    fn description(&self) -> &str {
        todo!()
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let closure: Closure = call.req(engine_state, stack, 0)?;

        // let from_io_error = IoError::factory(call.head, None);
        let wait_callback = || {
            crossterm::terminal::enable_raw_mode();
            // clear terminal events
            while crossterm::event::poll(Duration::from_secs(0)).unwrap() {
                // If there's an event, read it to remove it from the queue
                let _ = crossterm::event::read();
            }
            loop {
                if let Ok(Event::Key(k)) = crossterm::event::read() {
                    if k.kind == KeyEventKind::Press && k.code == KeyCode::Enter {
                        break;
                    }
                }
            }

            crossterm::terminal::disable_raw_mode();
        };

        let debugger = IrDebugger {
            wait_callback: Box::new(wait_callback),
        };

        let lock_err = |_| ShellError::GenericError {
            error: "Debugger Error".to_string(),
            msg: "could not lock debugger, poisoned mutex".to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        };

        engine_state
            .activate_debugger(Box::new(debugger))
            .map_err(lock_err)?;

        let result = ClosureEvalOnce::new(engine_state, stack, closure).run_with_input(input);

        // Return potential errors
        let pipeline_data = result?;

        // Collect the output
        let _ = pipeline_data.into_value(call.span());

        Ok(engine_state
            .deactivate_debugger()
            .map_err(lock_err)?
            .report(engine_state, call.span())?
            .into_pipeline_data())
    }
}
