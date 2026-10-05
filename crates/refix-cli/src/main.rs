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
    let xml = fs::read_to_string(&args.dictionary)
        .map_err(|error| format!("cannot read '{dictionary_path}': {error}"))?;
    let parsed = refix_dictionary::quickfix::parse(&xml)
        .map_err(|error| format!("cannot parse '{dictionary_path}': {error}"))?;
    for warning in &parsed.warnings {
        eprintln!("warning: {warning}");
    }

    let source = args
        .dictionary
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| args.dictionary.to_string_lossy());

    if let Some(output) = &args.rust {
        let generated = refix_codegen::rust::generate(&parsed.dictionary, &source)
            .map_err(|error| format!("cannot generate from '{dictionary_path}': {error}"))?;
        write(generated, output)?;
    }
    if let Some(output) = &args.python {
        let generated = refix_codegen::python::generate(&parsed.dictionary, &source)
            .map_err(|error| format!("cannot generate from '{dictionary_path}': {error}"))?;
        write(generated, output)?;
    }

    Ok(())
}

fn write(generated: Generated, output: &Path) -> Result<(), String> {
    for warning in &generated.warnings {
        eprintln!("warning: {warning}");
    }
    fs::write(output, generated.code)
        .map_err(|error| format!("cannot write '{}': {error}", output.display()))
}
