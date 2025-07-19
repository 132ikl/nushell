use itertools::Itertools;
use nu_engine::{command_prelude::*, get_full_help};
use nu_parser::{escape_for_script_arg, parse};
use nu_protocol::{
    ast::{Expr, Expression},
    engine::StateWorkingSet,
    report_parse_error,
};
use nu_utils::{escape_quote_string, stdout_write_all_and_flush};

// pub(crate) fn gather_commandline_args() -> (Vec<String>, Spanned<String>, Vec<String>) {
//     // Would be nice if we had a way to parse this. The first flags we see will be going to nushell
//     // then it'll be the script name
//     // then the args to the script
//     let mut args_to_nushell = Vec::from(["nu".into()]);
//     let mut script_name: Spanned<String> = String::new().into_spanned(Span::unknown());

//     let indexed_args = std::env::args().into_iter().fold(vec![], |acc, arg| {
//         let offset: usize = acc.last().map(|(val, off)| *off).unwrap_or(0);
//         acc.push((arg, offset));
//         acc
//     });
//     let args = indexed_args.into_iter();

//     let mut args_offset: usize = "nu".len();

//     // Mimic the behaviour of bash/zsh
//     if let Some(argv0) = args.next() {
//         if argv0.starts_with('-') {
//             args_to_nushell.push("--login".into());
//         }
//     }

//     while let Some(arg) = args.next() {
//         if !arg.starts_with('-') {
//             let len = arg.len();
//             script_name = arg.into_spanned(Span::new(args_offset, args_offset + len));
//             break;
//         }

//         args_offset += arg.len();

//         let flag_value = match arg.as_ref() {
//             "--commands" | "-c" | "--table-mode" | "-m" | "--error-style" | "-e" | "--execute"
//             | "--config" | "--env-config" | "-I" | "ide-ast" => {
//                 args.next().map(|a| escape_quote_string(&a))
//             }
//             #[cfg(feature = "plugin")]
//             "--plugin-config" => args.next().map(|a| escape_quote_string(&a)),
//             "--log-level"
//             | "--log-target"
//             | "--log-include"
//             | "--log-exclude"
//             | "--testbin"
//             | "--threads"
//             | "-t"
//             | "--include-path"
//             | "--lsp"
//             | "--ide-goto-def"
//             | "--ide-hover"
//             | "--ide-complete"
//             | "--ide-check"
//             | "--experimental-options" => args.next(),
//             #[cfg(feature = "plugin")]
//             "--plugins" => args.next(),
//             _ => None,
//         };

//         args_to_nushell.push(arg);

//         if let Some(flag_value) = flag_value {
//             args_to_nushell.push(flag_value);
//         }
//     }

//     let args_to_script = if !script_name.item.is_empty() {
//         args.map(|arg| escape_for_script_arg(&arg)).collect()
//     } else {
//         Vec::default()
//     };
//     (args_to_nushell, script_name, args_to_script)
// }

pub(crate) fn parse_commandline_args(
    engine_state: &mut EngineState,
) -> Result<NushellCliArgs, ShellError> {
    let mut args = std::env::args();
    let argv0 = args.next();
    let commandline = std::iter::once("nu".to_string()).chain(args).join(" ");

    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        working_set.add_decl(Box::new(Nu));

        let output = parse(&mut working_set, None, commandline.as_bytes(), false);
        if let Some(err) = working_set.parse_errors.first() {
            report_parse_error(&working_set, err);
            std::process::exit(1);
        }

        working_set.hide_decl(b"nu");
        (output, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    let mut stack = Stack::new();

    // We should have a successful parse now
    if let Some(pipeline) = block.pipelines.first() {
        if let Some(Expr::Call(call)) = pipeline.elements.first().map(|e| &e.expr.expr) {
            type StringArg = Option<Spanned<String>>;
            type ListArg = Option<Vec<Spanned<String>>>;

            let login_shell = if argv0.is_some_and(|arg| arg.starts_with("-")) {
                Some("--login".to_string().into_spanned(Span::unknown()))
            } else {
                call.get_named_arg("login")
            };

            let redirect_stdin = call.get_named_arg("stdin");
            let interactive_shell = call.get_named_arg("interactive");
            let commands: StringArg = call.get_flag(engine_state, &mut stack, "commands")?;
            let testbin: StringArg = call.get_flag(engine_state, &mut stack, "testbin")?;
            #[cfg(feature = "plugin")]
            let plugin_file: StringArg =
                call.get_flag(engine_state, &mut stack, "plugin-config")?;
            #[cfg(feature = "plugin")]
            let plugins: ListArg = call.get_flag(engine_state, &mut stack, "plugins")?;
            let no_config_file = call.get_named_arg("no-config-file");
            let no_history = call.get_named_arg("no-history");
            let no_std_lib = call.get_named_arg("no-std-lib");
            let config_file: StringArg = call.get_flag(engine_state, &mut stack, "config")?;
            let env_file: StringArg = call.get_flag(engine_state, &mut stack, "env-config")?;
            let log_level: StringArg = call.get_flag(engine_state, &mut stack, "log-level")?;
            let log_target: StringArg = call.get_flag(engine_state, &mut stack, "log-target")?;
            let log_include: ListArg = call.get_flag(engine_state, &mut stack, "log-include")?;
            let log_exclude: ListArg = call.get_flag(engine_state, &mut stack, "log-exclude")?;
            let execute: StringArg = call.get_flag(engine_state, &mut stack, "execute")?;
            let table_mode: Option<Value> =
                call.get_flag(engine_state, &mut stack, "table-mode")?;
            let error_style: Option<Value> =
                call.get_flag(engine_state, &mut stack, "error-style")?;
            let no_newline = call.get_named_arg("no-newline");
            let experimental_options: ListArg =
                call.get_flag(engine_state, &mut stack, "experimental-options")?;

            // ide flags
            let lsp = call.has_flag(engine_state, &mut stack, "lsp")?;
            let include_path: StringArg =
                call.get_flag(engine_state, &mut stack, "include-path")?;
            let ide_goto_def: Option<Value> =
                call.get_flag(engine_state, &mut stack, "ide-goto-def")?;
            let ide_hover: Option<Value> = call.get_flag(engine_state, &mut stack, "ide-hover")?;
            let ide_complete: Option<Value> =
                call.get_flag(engine_state, &mut stack, "ide-complete")?;
            let ide_check: Option<Value> = call.get_flag(engine_state, &mut stack, "ide-check")?;
            let ide_ast: StringArg = call.get_named_arg("ide-ast");

            let script_file: Option<Spanned<String>> = call.opt(engine_state, &mut stack, 0)?;
            let script_args: Vec<Spanned<String>> = call.rest(engine_state, &mut stack, 0)?;

            let help = call.has_flag(engine_state, &mut stack, "help")?;

            if help {
                let full_help = get_full_help(&Nu, engine_state, &mut stack);

                let _ = std::panic::catch_unwind(move || stdout_write_all_and_flush(full_help));

                std::process::exit(0);
            }

            if call.has_flag(engine_state, &mut stack, "version")? {
                let version = env!("CARGO_PKG_VERSION").to_string();
                let _ = std::panic::catch_unwind(move || {
                    stdout_write_all_and_flush(format!("{version}\n"))
                });

                std::process::exit(0);
            }

            return Ok(NushellCliArgs {
                script_file,
                script_args,
                redirect_stdin,
                login_shell,
                interactive_shell,
                commands,
                testbin,
                #[cfg(feature = "plugin")]
                plugin_file,
                #[cfg(feature = "plugin")]
                plugins,
                no_config_file,
                no_history,
                no_std_lib,
                config_file,
                env_file,
                log_level,
                log_target,
                log_include,
                log_exclude,
                execute,
                include_path,
                ide_goto_def,
                ide_hover,
                ide_complete,
                lsp,
                ide_check,
                ide_ast,
                table_mode,
                error_style,
                no_newline,
                experimental_options,
            });
        }
    }

    // Just give the help and exit if the above fails
    let full_help = get_full_help(&Nu, engine_state, &mut stack);
    print!("{full_help}");
    std::process::exit(1);
}

#[derive(Clone)]
pub(crate) struct NushellCliArgs {
    pub(crate) script_file: Option<Spanned<String>>,
    pub(crate) script_args: Vec<Spanned<String>>,
    pub(crate) redirect_stdin: Option<Spanned<String>>,
    pub(crate) login_shell: Option<Spanned<String>>,
    pub(crate) interactive_shell: Option<Spanned<String>>,
    pub(crate) commands: Option<Spanned<String>>,
    pub(crate) testbin: Option<Spanned<String>>,
    #[cfg(feature = "plugin")]
    pub(crate) plugin_file: Option<Spanned<String>>,
    #[cfg(feature = "plugin")]
    pub(crate) plugins: Option<Vec<Spanned<String>>>,
    pub(crate) no_config_file: Option<Spanned<String>>,
    pub(crate) no_history: Option<Spanned<String>>,
    pub(crate) no_std_lib: Option<Spanned<String>>,
    pub(crate) config_file: Option<Spanned<String>>,
    pub(crate) env_file: Option<Spanned<String>>,
    pub(crate) log_level: Option<Spanned<String>>,
    pub(crate) log_target: Option<Spanned<String>>,
    pub(crate) log_include: Option<Vec<Spanned<String>>>,
    pub(crate) log_exclude: Option<Vec<Spanned<String>>>,
    pub(crate) execute: Option<Spanned<String>>,
    pub(crate) table_mode: Option<Value>,
    pub(crate) error_style: Option<Value>,
    pub(crate) no_newline: Option<Spanned<String>>,
    pub(crate) include_path: Option<Spanned<String>>,
    pub(crate) lsp: bool,
    pub(crate) ide_goto_def: Option<Value>,
    pub(crate) ide_hover: Option<Value>,
    pub(crate) ide_complete: Option<Value>,
    pub(crate) ide_check: Option<Value>,
    pub(crate) ide_ast: Option<Spanned<String>>,
    pub(crate) experimental_options: Option<Vec<Spanned<String>>>,
}

#[derive(Clone)]
struct Nu;

impl Command for Nu {
    fn name(&self) -> &str {
        "nu"
    }

    fn signature(&self) -> Signature {
        let mut signature = Signature::build("nu")
            .description("The nushell language and shell.")
            .named(
                "commands",
                SyntaxShape::String,
                "run the given commands and then exit",
                Some('c'),
            )
            .named(
                "execute",
                SyntaxShape::String,
                "run the given commands and then enter an interactive shell",
                Some('e'),
            )
            .named(
                "include-path",
                SyntaxShape::String,
                "set the NU_LIB_DIRS for the given script (delimited by char record_sep ('\x1e'))",
                Some('I'),
            )
            .switch("interactive", "start as an interactive shell", Some('i'))
            .switch("login", "start as a login shell", Some('l'))
            .named(
                "table-mode",
                SyntaxShape::String,
                "the table mode to use. rounded is default.",
                Some('m'),
            )
            .named(
                "error-style",
                SyntaxShape::String,
                "the error style to use (fancy or plain). default: fancy",
                None,
            )
            .switch("no-newline", "print the result for --commands(-c) without a newline", None)
            .switch(
                "no-config-file",
                "start with no config file and no env file",
                Some('n'),
            )
            .switch(
                "no-history",
                "disable reading and writing to command history",
                None,
            )
            .switch("no-std-lib", "start with no standard library", None)
            .named(
                "threads",
                SyntaxShape::Int,
                "threads to use for parallel commands",
                Some('t'),
            )
            .switch("version", "print the version", Some('v'))
            .named(
                "config",
                SyntaxShape::Filepath,
                "start with an alternate config file",
                None,
            )
            .named(
                "env-config",
                SyntaxShape::Filepath,
                "start with an alternate environment config file",
                None,
            )
            .switch(
               "lsp",
               "start nu's language server protocol",
               None,
            )
           .named(
                "ide-goto-def",
                SyntaxShape::Int,
                "go to the definition of the item at the given position",
                None,
            )
            .named(
                "ide-hover",
                SyntaxShape::Int,
                "give information about the item at the given position",
                None,
             )
            .named(
                "ide-complete",
                SyntaxShape::Int,
                "list completions for the item at the given position",
                None,
            )
            .named(
                "ide-check",
                SyntaxShape::Int,
                "run a diagnostic check on the given source and limit number of errors returned to provided number",
                None,
            )
            .switch("ide-ast", "generate the ast on the given source", None);

        #[cfg(feature = "plugin")]
        {
            signature = signature
                .named(
                    "plugin-config",
                    SyntaxShape::Filepath,
                    "start with an alternate plugin registry file",
                    None,
                )
                .named(
                    "plugins",
                    SyntaxShape::List(Box::new(SyntaxShape::Filepath)),
                    "list of plugin executable files to load, separately from the registry file",
                    None,
                )
        }

        signature = signature
            .named(
                "log-level",
                SyntaxShape::String,
                "log level for diagnostic logs (error, warn, info, debug, trace). Off by default",
                None,
            )
            .named(
                "log-target",
                SyntaxShape::String,
                "set the target for the log to output. stdout, stderr(default), mixed or file",
                None,
            )
            .named(
                "log-include",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "set the Rust module prefixes to include in the log output. default: [nu]",
                None,
            )
            .named(
                "log-exclude",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "set the Rust module prefixes to exclude from the log output",
                None,
            )
            .switch(
                "stdin",
                "redirect standard input to a command (with `-c`) or a script file",
                None,
            )
            .named(
                "testbin",
                SyntaxShape::String,
                "run internal test binary",
                None,
            )
            .named(
                "experimental-options",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                r#"enable or disable experimental options, use `"all"` to set all active options"#,
                None,
            )
            .optional(
                "script file",
                SyntaxShape::Filepath,
                "name of the optional script file to run",
            )
            .rest(
                "script args",
                SyntaxShape::String,
                "parameters to the script file",
            )
            .category(Category::System);

        signature
    }

    fn description(&self) -> &str {
        "The nushell language and shell."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(get_full_help(self, engine_state, stack), call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
        vec![
            Example {
                description: "Run a script",
                example: "nu myfile.nu",
                result: None,
            },
            Example {
                description: "Run nushell interactively (as a shell or REPL)",
                example: "nu",
                result: None,
            },
        ]
    }
}
