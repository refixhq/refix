use clap::{ArgGroup, Args, Parser, Subcommand};
use refix_codegen::Generated;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Command-line tools for the ReFIX engine.
#[derive(Parser)]
#[command(name = "refix", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generates typed messages from a FIX dictionary.
    Codegen(Codegen),
}

#[derive(Args)]
#[command(group(ArgGroup::new("outputs").required(true).multiple(true)))]
struct Codegen {
    /// QuickFIX-format dictionary XML to read.
    dictionary: PathBuf,
    /// Transport dictionary XML with the header, trailer and session
    /// messages, such as FIXT11.xml for FIX 5.0.
    #[arg(long, value_name = "PATH")]
    transport: Option<PathBuf>,
    /// Path of the generated Rust module.
    #[arg(long, group = "outputs", value_name = "PATH")]
    rust: Option<PathBuf>,
    /// Path of the generated Python module.
    #[arg(long, group = "outputs", value_name = "PATH")]
    python: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match &cli.command {
        Command::Codegen(args) => codegen(args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn codegen(args: &Codegen) -> Result<(), String> {
    let dictionary_path = args.dictionary.display();
    let xml = read(&args.dictionary)?;
    let mut source = file_name(&args.dictionary);
    let parsed = match &args.transport {
        Some(transport) => {
            let transport_xml = read(transport)?;
            source = format!("{source} and {}", file_name(transport));
            refix_dictionary::quickfix::parse_with_transport(&xml, &transport_xml).map_err(
                |error| {
                    format!(
                        "cannot parse '{dictionary_path}' with '{}': {error}",
                        transport.display()
                    )
                },
            )?
        }
        None => refix_dictionary::quickfix::parse(&xml)
            .map_err(|error| format!("cannot parse '{dictionary_path}': {error}"))?,
    };
    for warning in &parsed.warnings {
        eprintln!("warning: {warning}");
    }
    if parsed.dictionary.header().next().is_none() {
        eprintln!(
            "warning: the dictionary has no header, which from FIX 5.0 is in a transport \
             dictionary such as FIXT11.xml, given with --transport"
        );
    }

    if let Some(output) = &args.rust {
        let generated = refix_codegen::rust::generate(&parsed.dictionary, &source)
            .map_err(|error| format!("cannot generate from '{dictionary_path}': {error}"))?;
        write(generated, output, "rust")?;
    }
    if let Some(output) = &args.python {
        let generated = refix_codegen::python::generate(&parsed.dictionary, &source)
            .map_err(|error| format!("cannot generate from '{dictionary_path}': {error}"))?;
        write(generated, output, "python")?;
    }

    Ok(())
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read '{}': {error}", path.display()))
}

/// The name of the file at `path`, for the generated module's first line.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| path.to_string_lossy())
        .into_owned()
}

fn write(generated: Generated, output: &Path, language: &str) -> Result<(), String> {
    for warning in &generated.warnings {
        eprintln!("warning: {language}: {warning}");
    }
    fs::write(output, generated.code)
        .map_err(|error| format!("cannot write '{}': {error}", output.display()))
}
