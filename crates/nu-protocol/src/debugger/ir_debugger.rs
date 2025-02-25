use crate::{debugger::Debugger, engine::EngineState, ir::IrBlock, PipelineData, ShellError};

pub struct IrDebugger {
    pub wait_callback: Box<dyn Fn() + Send>,
}

impl std::fmt::Debug for IrDebugger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrDebugger")
            .field("wait_callback", &"<closure>")
            .finish()
    }
}

impl Debugger for IrDebugger {
    #[allow(unused_variables)]
    fn enter_instruction(
        &mut self,
        engine_state: &EngineState,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineData],
    ) {
        let inst = &ir_block.instructions[instruction_index];
        println!("{}", inst.display(engine_state, &ir_block.data));

        (self.wait_callback)();
    }
}
