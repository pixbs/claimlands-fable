//! Machine setup, mobile builds and scaffolding.

use std::fs;

use anyhow::{Context, Result, bail};
use xshell::{Shell, cmd};

use crate::root;
use crate::work::slug_of;

/// Whether a program is on `PATH` (cargo subcommands are checked as `cargo <name>`).
pub fn has(sh: &Shell, program: &str) -> bool {
    if let Some(sub) = program.strip_prefix("cargo-") {
        return cmd!(sh, "cargo {sub} --version")
            .quiet()
            .ignore_stderr()
            .read()
            .is_ok();
    }
    cmd!(sh, "{program} --version")
        .quiet()
        .ignore_stderr()
        .read()
        .is_ok()
}

/// Targets, cargo tools, git hooks. Idempotent; prints what is still missing.
pub fn setup(sh: &Shell) -> Result<()> {
    println!("== targets");
    cmd!(sh, "rustup target add wasm32-unknown-unknown aarch64-linux-android aarch64-apple-ios aarch64-apple-ios-sim").run()?;
    println!("== cargo tools");
    for (tool, crate_name) in [
        ("cargo-nextest", "cargo-nextest"),
        ("cargo-deny", "cargo-deny"),
        ("cargo-llvm-cov", "cargo-llvm-cov"),
        ("cargo-ndk", "cargo-ndk"),
    ] {
        if has(sh, tool) {
            println!("   {tool}: present");
        } else {
            println!("   {tool}: installing");
            cmd!(sh, "cargo install {crate_name} --locked").run()?;
        }
    }
    println!("== git hooks");
    cmd!(sh, "git config core.hooksPath .githooks").run()?;
    println!("   core.hooksPath = .githooks");
    println!("== optional tools");
    for (tool, hint) in [
        (
            "wasm-opt",
            "binaryen: apt install binaryen / brew install binaryen / https://github.com/WebAssembly/binaryen/releases",
        ),
        ("node", "Node 22+: needed only to regenerate fixtures"),
        ("gh", "GitHub CLI: needed for cargo xtask next/take"),
        ("gradle", "Android APK builds; CI has it"),
    ] {
        println!(
            "   {tool}: {}",
            if has(sh, tool) {
                "present".to_owned()
            } else {
                format!("missing ({hint})")
            }
        );
    }
    if has(sh, "gh") && !has(sh, "gh-stack") {
        println!("   gh stack: run `gh extension install github/gh-stack` for stacked PRs");
    }
    Ok(())
}

/// `cargo ndk` into the Gradle project's `jniLibs`, then the debug APK when Gradle is present.
pub fn android(sh: &Shell, release: bool) -> Result<()> {
    if !has(sh, "cargo-ndk") {
        bail!("cargo-ndk is not installed; run cargo xtask setup");
    }
    let root = root();
    let jni = root.join("platforms/android/app/src/main/jniLibs");
    println!("== android: cargo ndk");
    let mut args = vec![
        "ndk",
        "-t",
        "arm64-v8a",
        "-o",
        jni.to_str().context("path")?,
        "build",
        "-p",
        "cl-app",
        "--lib",
    ];
    if release {
        args.push("--release");
    }
    cmd!(sh, "cargo {args...}").run()?;
    let gradle_dir = root.join("platforms/android");
    if has(sh, "gradle") {
        println!("== android: gradle assembleDebug");
        let _d = sh.push_dir(&gradle_dir);
        cmd!(sh, "gradle assembleDebug --no-daemon -q").run()?;
        println!("== android: platforms/android/app/build/outputs/apk/debug/app-debug.apk");
    } else {
        println!(
            "== android: gradle not installed; library is in {} (CI builds the APK)",
            jni.display()
        );
    }
    Ok(())
}

/// The iOS static library for device and simulator; the Xcode project only on macOS.
pub fn ios(sh: &Shell, release: bool) -> Result<()> {
    println!("== ios: cargo build (device + simulator)");
    for target in ["aarch64-apple-ios", "aarch64-apple-ios-sim"] {
        let mut args = vec!["build", "-p", "cl-app", "--lib", "--target", target];
        if release {
            args.push("--release");
        }
        cmd!(sh, "cargo {args...}").run()?;
    }
    if cfg!(target_os = "macos") {
        let root = root();
        let _d = sh.push_dir(root.join("platforms/ios"));
        if has(sh, "xcodegen") {
            println!("== ios: xcodegen");
            cmd!(sh, "xcodegen generate").run()?;
            println!("== ios: xcodebuild (unsigned)");
            let config = if release { "Release" } else { "Debug" };
            cmd!(sh, "xcodebuild -project ClaimLands.xcodeproj -scheme ClaimLands -configuration {config} -sdk iphoneos CODE_SIGNING_ALLOWED=NO build").run()?;
        } else {
            println!("== ios: xcodegen not installed (brew install xcodegen)");
        }
    } else {
        println!("== ios: libraries built; the Xcode project is generated and built on macOS only");
    }
    Ok(())
}

/// A crate directory from the template with README sections and lints wired.
pub fn new_crate(sh: &Shell, name: &str) -> Result<()> {
    if !name.starts_with("cl-")
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit())
    {
        bail!("crate names look like cl-<feature>");
    }
    let dir = root().join("crates").join(name);
    if dir.exists() {
        bail!("{} exists", dir.display());
    }
    fs::create_dir_all(dir.join("src"))?;
    let template = fs::read_to_string(root().join("docs/templates/crate-README.md"))?;
    fs::write(dir.join("README.md"), template.replace("{{name}}", name))?;
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\ndescription = \"TODO(#0): one line\"\nversion.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nlicense.workspace = true\nrepository.workspace = true\npublish.workspace = true\n\n[dependencies]\ncl-model = {{ workspace = true }}\n\n[lints]\nworkspace = true\n"
        ),
    )?;
    fs::write(
        dir.join("src/lib.rs"),
        "//! One sentence on what this crate owns.\n#![forbid(unsafe_code)]\n",
    )?;
    println!(
        "created crates/{name}; add it to [workspace.dependencies] and to the allow-list in xtask/src/lint.rs and docs/architecture.md"
    );
    let _ = sh;
    Ok(())
}

/// The next ADR number with the template filled in.
pub fn new_adr(sh: &Shell, title: &str) -> Result<()> {
    let dir = root().join("docs/adr");
    let mut max = 0u32;
    for entry in fs::read_dir(&dir)? {
        let name = entry?.file_name().to_string_lossy().to_string();
        if let Some(n) = name.get(..4).and_then(|s| s.parse::<u32>().ok()) {
            max = max.max(n);
        }
    }
    let number = max + 1;
    let file = dir.join(format!("{number:04}-{}.md", slug_of(title)));
    let template = fs::read_to_string(root().join("docs/templates/adr.md"))?;
    let date = cmd!(sh, "git log -1 --format=%cd --date=short")
        .ignore_status()
        .read()
        .unwrap_or_default();
    let date = if date.trim().is_empty() {
        "YYYY-MM-DD".to_owned()
    } else {
        date.trim().to_owned()
    };
    fs::write(
        &file,
        template
            .replace("{{number}}", &format!("{number:04}"))
            .replace("{{title}}", title)
            .replace("{{date}}", &date),
    )?;
    println!("created {}", file.display());
    Ok(())
}
