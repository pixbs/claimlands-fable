//! Repository task runner: `cargo xtask <command>`. One binary, no extra installs, the same
//! commands locally and in CI. `AGENTS.md` lists when to run which.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod lint;
mod tools;
mod web;
mod work;

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use xshell::{Shell, cmd};

#[derive(Parser)]
#[command(name = "xtask", about = "Claim Lands repository tasks", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Install toolchain targets, cargo tools and the git hooks.
    Setup,
    /// Every gate CI runs: fmt, clippy (host + wasm), tests, docs, deny, lint-repo.
    Check,
    /// Format the workspace.
    Fmt,
    /// Clippy with warnings denied, for the host and for wasm32.
    Clippy,
    /// Tests (nextest when installed, cargo test otherwise).
    Test,
    /// Repository lints: crate dependency graph, prose, unreferenced work markers, banned words, crate READMEs.
    LintRepo,
    /// Regenerate `fixtures/` from the frozen prototype (`--full` also dumps complete arrays).
    Fixtures {
        /// Also write complete arrays to reference/harness/out (gitignored).
        #[arg(long)]
        full: bool,
    },
    /// Build the web bundle into `dist/`.
    Web {
        /// Debug build (default is release with wasm-opt when available).
        #[arg(long)]
        debug: bool,
    },
    /// Serve a directory over HTTP (default `dist/` on port 8080).
    Serve {
        /// Port.
        #[arg(long, default_value_t = 8080)]
        port: u16,
        /// Directory.
        #[arg(long, default_value = "dist")]
        dir: PathBuf,
    },
    /// Build the Android library (`cargo ndk`) and, when Gradle is available, the debug APK.
    Android {
        /// Release build.
        #[arg(long)]
        release: bool,
    },
    /// Build the iOS static library and, on macOS, the Xcode project.
    Ios {
        /// Release build.
        #[arg(long)]
        release: bool,
    },
    /// List issues an agent may take: open, `ready`, unassigned, unblocked, earliest milestone first.
    Next,
    /// Claim an issue (the first available when omitted), create its worktree and branch.
    Take {
        /// Issue number.
        number: Option<u64>,
    },
    /// Create a worktree and branch for an issue without claiming it on GitHub.
    Worktree {
        /// Issue number.
        number: u64,
        /// Branch slug; derived from the issue title when omitted.
        #[arg(long)]
        slug: Option<String>,
    },
    /// Scaffold a crate under `crates/` from the template.
    NewCrate {
        /// Crate name, with the `cl-` prefix.
        name: String,
    },
    /// Scaffold an ADR under `docs/adr/`.
    NewAdr {
        /// Title, e.g. "Use event sourcing for replays".
        title: String,
    },
}

/// Repository root (the workspace root, two levels above this crate's manifest).
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let sh = Shell::new()?;
    sh.change_dir(root());
    match cli.cmd {
        Cmd::Setup => tools::setup(&sh),
        Cmd::Check => check(&sh),
        Cmd::Fmt => Ok(cmd!(sh, "cargo fmt --all").run()?),
        Cmd::Clippy => clippy(&sh),
        Cmd::Test => test(&sh),
        Cmd::LintRepo => lint::run(&sh),
        Cmd::Fixtures { full } => {
            let mut args = vec!["reference/harness/extract.mjs"];
            if full {
                args.push("--full");
            }
            Ok(cmd!(sh, "node {args...}").run()?)
        }
        Cmd::Web { debug } => web::build(&sh, !debug),
        Cmd::Serve { port, dir } => web::serve(&dir, port),
        Cmd::Android { release } => tools::android(&sh, release),
        Cmd::Ios { release } => tools::ios(&sh, release),
        Cmd::Next => work::next(&sh),
        Cmd::Take { number } => work::take(&sh, number),
        Cmd::Worktree { number, slug } => work::worktree(&sh, number, slug),
        Cmd::NewCrate { name } => tools::new_crate(&sh, &name),
        Cmd::NewAdr { title } => tools::new_adr(&sh, &title),
    }
}

fn check(sh: &Shell) -> Result<()> {
    println!("== fmt");
    cmd!(sh, "cargo fmt --all --check").run()?;
    clippy(sh)?;
    test(sh)?;
    println!("== doc");
    cmd!(sh, "cargo doc --workspace --no-deps")
        .env("RUSTDOCFLAGS", "-D warnings")
        .run()?;
    if tools::has(sh, "cargo-deny") {
        println!("== deny");
        cmd!(sh, "cargo deny check").run()?;
    } else {
        println!("== deny skipped (cargo-deny not installed; run cargo xtask setup)");
    }
    lint::run(sh)?;
    println!("== all gates passed");
    Ok(())
}

fn clippy(sh: &Shell) -> Result<()> {
    println!("== clippy (host)");
    cmd!(sh, "cargo clippy --workspace --all-targets -- -D warnings").run()?;
    println!("== clippy (wasm32)");
    cmd!(
        sh,
        "cargo clippy -p cl-app --target wasm32-unknown-unknown -- -D warnings"
    )
    .run()?;
    Ok(())
}

fn test(sh: &Shell) -> Result<()> {
    println!("== test");
    if tools::has(sh, "cargo-nextest") {
        cmd!(sh, "cargo nextest run --workspace").run()?;
    } else {
        cmd!(sh, "cargo test --workspace").run()?;
    }
    Ok(())
}
