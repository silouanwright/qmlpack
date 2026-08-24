use clap::{Parser, Subcommand};
use omapack::github::GitHubClient;
use omapack::resolver::Resolver;
use omapack::workspace;
use omapack::{MANIFEST_LIMIT, OmapackError, PackageManifest, Source};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "omapack",
    version,
    about = "Review-first source package management for Omarchy plugins"
)]
struct Cli {
    /// Plugin project directory.
    #[arg(long, global = true, default_value = ".")]
    project: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create an empty Omarchy project manifest.
    Init,
    /// Prepare a package addition for review without changing the project.
    Add { label: String, source: String },
    /// Prepare one direct dependency at a new exact version or commit.
    Update {
        label: String,
        #[arg(long)]
        to: String,
    },
    /// Prepare removal of a direct dependency.
    Remove { label: String },
    /// Show the prepared candidate review.
    Diff,
    /// Apply the previously prepared and reviewed candidate.
    Apply {
        /// Replace locally modified managed files.
        #[arg(long)]
        force: bool,
    },
    /// Verify installed source against omapack.lock.
    Verify,
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
        Command::Init => {
            workspace::initialize(&cli.project)?;
            println!("Created {}", cli.project.join("omapack.json").display());
        }
        Command::Add { label, source } => {
            let mut project = workspace::read_project(&cli.project)?;
            if project.dependencies.contains_key(&label) {
                return Err(OmapackError(format!(
                    "{label} already exists; use omapack update"
                )));
            }
            project.dependencies.insert(label, Source::parse(&source)?);
            prepare(&cli.project, &project)?;
        }
        Command::Update { label, to } => {
            let mut project = workspace::read_project(&cli.project)?;
            let old = project
                .dependencies
                .get(&label)
                .ok_or_else(|| OmapackError(format!("unknown direct dependency: {label}")))?;
            let path = if old.package_path.is_empty() {
                String::new()
            } else {
                format!("/{}", old.package_path)
            };
            let updated = Source::parse(&format!(
                "github:{}/{}{}@{to}",
                old.owner, old.repository, path
            ))?;
            project.dependencies.insert(label, updated);
            prepare(&cli.project, &project)?;
        }
        Command::Remove { label } => {
            let mut project = workspace::read_project(&cli.project)?;
            if project.dependencies.remove(&label).is_none() {
                return Err(OmapackError(format!("unknown direct dependency: {label}")));
            }
            prepare(&cli.project, &project)?;
        }
        Command::Diff => print!("{}", workspace::candidate_review(&cli.project)?),
        Command::Apply { force } => {
            workspace::apply(&cli.project, force)?;
            println!("Applied reviewed Omapack candidate.");
        }
        Command::Verify => {
            workspace::verify(&cli.project)?;
            println!("Installed packages match omapack.lock.");
        }
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

fn prepare(
    root: &std::path::Path,
    project: &omapack::project::ProjectManifest,
) -> Result<(), OmapackError> {
    let token = env::var("GITHUB_TOKEN").ok();
    let mut client = GitHubClient::new(token.as_deref())?;
    let graph = Resolver::new(&mut client).resolve(&project.dependencies)?;
    let review = workspace::prepare(root, project, &graph)?;
    print!("{review}");
    println!("Review `.omapack/candidate/`, then run `omapack apply`.");
    Ok(())
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("omapack: {error}");
        std::process::exit(1);
    }
}
