use clap::{Parser, Subcommand};
use omapack::{MANIFEST_LIMIT, OmapackError, PackageManifest, Source};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "omapack",
    version,
    about = "Review-first source package management for Omarchy plugins"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a package manifest.
    CheckManifest {
        #[arg(default_value = "omapack.json")]
        path: PathBuf,
    },
    #[command(hide = true)]
    InspectSource { source: String },
}

fn run(cli: Cli) -> Result<(), OmapackError> {
    match cli.command {
        Command::CheckManifest { path } => {
            let payload = fs::read(&path)?;
            if payload.len() > MANIFEST_LIMIT {
                return Err(OmapackError(format!(
                    "{} exceeds {MANIFEST_LIMIT} bytes",
                    path.display()
                )));
            }
            let manifest = PackageManifest::parse(&payload)?;
            println!(
                "{}: {} files, {} dependencies",
                manifest.name,
                manifest.files.len(),
                manifest.dependencies.len()
            );
        }
        Command::InspectSource { source } => println!("{}", Source::parse(&source)?.canonical()),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("omapack: {error}");
        std::process::exit(1);
    }
}
