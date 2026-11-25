use clap::{ArgGroup, Args, Parser, Subcommand};
use refix_codegen::Package;
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
    /// Directory of the generated Rust module, named as the module is.
    #[arg(long, group = "outputs", value_name = "DIRECTORY")]
    rust: Option<PathBuf>,
    /// Directory of the generated Python package, named as it is imported.
    #[arg(long, group = "outputs", value_name = "DIRECTORY")]
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
        if let Some(name) = unusable_name(output) {
            return Err(format!(
                "--rust takes the directory of a module, and '{name}' is not a name a rust \
                 module can have"
            ));
        }
        let package = refix_codegen::rust::generate(&parsed.dictionary, &source)
            .map_err(|error| format!("cannot generate from '{dictionary_path}': {error}"))?;
        write(&package, output, "rust")?;
    }
    if let Some(output) = &args.python {
        if let Some(name) = unusable_name(output) {
            return Err(format!(
                "--python takes the directory of a package, and '{name}' is not a name python \
                 can import"
            ));
        }
        let package = refix_codegen::python::generate(&parsed.dictionary, &source)
            .map_err(|error| format!("cannot generate from '{dictionary_path}': {error}"))?;
        write(&package, output, "python")?;
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

/// The name of an output directory when no module or package can have it,
/// such as the `fix44.rs` or `fix44.py` an earlier release took.
fn unusable_name(directory: &Path) -> Option<String> {
    let name = directory.file_name()?.to_string_lossy();
    let mut characters = name.chars();
    let starts_a_name = characters
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    let is_a_name = starts_a_name && characters.all(|c| c.is_ascii_alphanumeric() || c == '_');
    (!is_a_name).then(|| name.into_owned())
}

fn write(package: &Package, output: &Path, language: &str) -> Result<(), String> {
    for warning in &package.warnings {
        eprintln!("warning: {language}: {warning}");
    }
    package.write(output).map_err(|error| error.to_string())
}
