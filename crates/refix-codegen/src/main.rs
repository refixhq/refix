use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

/// Generates typed message wrappers from a FIX dictionary.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// QuickFIX-format dictionary XML to read.
    dictionary: PathBuf,
    /// Path of the generated Rust module.
    output: PathBuf,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), String> {
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
    let generated = refix_codegen::generate(&parsed.dictionary, &source)
        .map_err(|error| format!("cannot generate from '{dictionary_path}': {error}"))?;
    fs::write(&args.output, generated)
        .map_err(|error| format!("cannot write '{}': {error}", args.output.display()))?;

    Ok(())
}
