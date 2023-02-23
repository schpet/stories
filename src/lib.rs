use anyhow::anyhow;
use mdcat::push_tty;
use mdcat::terminal::TerminalProgram;

use mdcat::Settings;
use pulldown_cmark::{Options, Parser};
use syntect::parsing::SyntaxSet;

pub fn print_markdown(text: &str) -> anyhow::Result<()> {
    let parser = Parser::new_ext(
        text,
        Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH,
    );

    let settings: Settings = mdcat::Settings {
        terminal_capabilities: TerminalProgram::Ansi.capabilities(),
        terminal_size: mdcat::terminal::TerminalSize::default(),
        resource_access: mdcat::ResourceAccess::LocalOnly,
        syntax_set: SyntaxSet::load_defaults_newlines(),
    };
    let env = mdcat::Environment::for_local_directory(&std::env::current_dir().unwrap()).unwrap();

    let stdout = std::io::stdout();
    let mut output = stdout.lock();

    push_tty(&settings, &env, &mut output, parser).or_else(|error| {
        if error.kind() == std::io::ErrorKind::BrokenPipe {
            Ok(())
        } else {
            Err(anyhow!("Cannot render markdown to stdout: {:?}", error))
        }
    })
}
