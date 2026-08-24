use clap::{Parser, Subcommand};
use qmlpack::github::GitHubClient;
use qmlpack::resolver::Resolver;
use qmlpack::workspace;
use qmlpack::{MANIFEST_LIMIT, PackageManifest, QmlpackError, Source};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "qmlpack",
    version,
    about = "Review-first source package management for QML projects"
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
    /// Create an empty Qmlpack project manifest.
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
    /// Verify installed source against qmlpack.lock.
    Verify,
    /// Validate a package manifest.
    CheckManifest {
        #[arg(default_value = "qmlpack.json")]
        path: PathBuf,
    },
    #[command(hide = true)]
    InspectSource { source: String },
}

fn run(cli: Cli) -> Result<(), QmlpackError> {
    match cli.command {
        Command::Init => {
            workspace::initialize(&cli.project)?;
            println!("Created {}", cli.project.join("qmlpack.json").display());
        }
        Command::Add { label, source } => {
            let mut project = workspace::read_project(&cli.project)?;
            if project.dependencies.contains_key(&label) {
                return Err(QmlpackError(format!(
                    "{label} already exists; use qmlpack update"
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
                .ok_or_else(|| QmlpackError(format!("unknown direct dependency: {label}")))?;
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
                return Err(QmlpackError(format!("unknown direct dependency: {label}")));
            }
            prepare(&cli.project, &project)?;
        }
        Command::Diff => print!("{}", workspace::candidate_review(&cli.project)?),
        Command::Apply { force } => {
            workspace::apply(&cli.project, force)?;
            println!("Applied reviewed Qmlpack candidate.");
        }
        Command::Verify => {
            workspace::verify(&cli.project)?;
            println!("Installed packages match qmlpack.lock.");
        }
        Command::CheckManifest { path } => {
            let payload = fs::read(&path)?;
            if payload.len() > MANIFEST_LIMIT {
                return Err(QmlpackError(format!(
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
    project: &qmlpack::project::ProjectManifest,
) -> Result<(), QmlpackError> {
    let token = env::var("GITHUB_TOKEN").ok();
    let mut client = GitHubClient::new(token.as_deref())?;
    let graph = Resolver::new(&mut client).resolve(&project.dependencies)?;
    let review = workspace::prepare(root, project, &graph)?;
    print!("{review}");
    println!("Review `.qmlpack/candidate/`, then run `qmlpack apply`.");
    Ok(())
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("qmlpack: {error}");
        std::process::exit(1);
    }
}
