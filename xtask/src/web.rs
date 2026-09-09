//! The web bundle: compile `cl-app` to wasm, run `wasm-bindgen`, copy the page shell and the
//! frozen prototype, optionally shrink with `wasm-opt`. `serve` hosts the result locally.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use xshell::{Shell, cmd};

use crate::root;
use crate::tools::has;

/// Builds `dist/`.
pub fn build(sh: &Shell, release: bool) -> Result<()> {
    let root = root();
    let dist = root.join("dist");
    let profile_dir = if release { "release" } else { "debug" };
    println!("== web: cargo build ({profile_dir})");
    let mut args = vec![
        "build",
        "-p",
        "cl-app",
        "--target",
        "wasm32-unknown-unknown",
        "--lib",
    ];
    if release {
        args.push("--release");
    }
    cmd!(sh, "cargo {args...}").run()?;
    let wasm = root
        .join("target/wasm32-unknown-unknown")
        .join(profile_dir)
        .join("cl_app.wasm");
    if !wasm.exists() {
        bail!("expected {} after the build", wasm.display());
    }

    ensure_wasm_bindgen(sh)?;
    if dist.exists() {
        fs::remove_dir_all(&dist)?;
    }
    fs::create_dir_all(&dist)?;
    println!("== web: wasm-bindgen");
    cmd!(
        sh,
        "wasm-bindgen --target web --no-typescript --out-dir {dist} --out-name claimlands {wasm}"
    )
    .run()?;

    println!("== web: page shell and reference");
    for entry in fs::read_dir(root.join("platforms/web"))? {
        let entry = entry?;
        fs::copy(entry.path(), dist.join(entry.file_name()))?;
    }
    fs::create_dir_all(dist.join("reference"))?;
    fs::copy(
        root.join("reference/hex-planet.html"),
        dist.join("reference/index.html"),
    )?;
    let sha = cmd!(sh, "git rev-parse --short HEAD")
        .read()
        .unwrap_or_else(|_| "unknown".into());
    fs::write(dist.join("version.txt"), format!("{sha}\n"))?;

    let bg = dist.join("claimlands_bg.wasm");
    if release && has(sh, "wasm-opt") {
        println!("== web: wasm-opt");
        cmd!(sh, "wasm-opt -O2 --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals --enable-reference-types {bg} -o {bg}").run()?;
    } else if release {
        println!("== web: wasm-opt not installed, bundle left unoptimised");
    }
    let size = fs::metadata(&bg)?.len();
    println!(
        "== web: dist/ ready ({} KiB wasm, commit {sha})",
        size / 1024
    );
    Ok(())
}

/// The `wasm-bindgen` CLI must match the crate version in `Cargo.lock` exactly.
fn ensure_wasm_bindgen(sh: &Shell) -> Result<()> {
    let lock = fs::read_to_string(root().join("Cargo.lock"))?;
    let wanted = lock
        .split("[[package]]")
        .find(|p| p.contains("name = \"wasm-bindgen\"\n"))
        .and_then(|p| p.lines().find_map(|l| l.strip_prefix("version = \"")))
        .map(|v| v.trim_end_matches('"').to_owned())
        .context("wasm-bindgen version in Cargo.lock")?;
    let installed = cmd!(sh, "wasm-bindgen --version")
        .ignore_status()
        .read()
        .unwrap_or_default();
    if installed.trim().ends_with(&wanted) {
        return Ok(());
    }
    println!(
        "== web: installing wasm-bindgen-cli {wanted} (found {:?})",
        installed.trim()
    );
    cmd!(
        sh,
        "cargo install wasm-bindgen-cli --version {wanted} --locked"
    )
    .run()?;
    Ok(())
}

/// Minimal static file server for local review; `.wasm` gets its MIME type, nothing is cached.
pub fn serve(dir: &Path, port: u16) -> Result<()> {
    let dir: PathBuf = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        root().join(dir)
    };
    if !dir.join("index.html").exists() {
        bail!(
            "{} has no index.html; run cargo xtask web first",
            dir.display()
        );
    }
    let server = tiny_http::Server::http(("0.0.0.0", port))
        .map_err(|e| anyhow::anyhow!("bind port {port}: {e}"))?;
    println!(
        "serving {} at http://localhost:{port}/ (reference at /reference/) — Ctrl+C to stop",
        dir.display()
    );
    for request in server.incoming_requests() {
        let url = request.url().split('?').next().unwrap_or("/");
        let mut rel = url.trim_start_matches('/').to_owned();
        if rel.is_empty() || rel.ends_with('/') {
            rel.push_str("index.html");
        }
        let path = dir.join(&rel);
        let response = match fs::File::open(&path) {
            Ok(mut file) => {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                let mime = match path.extension().and_then(|e| e.to_str()) {
                    Some("html") => "text/html; charset=utf-8",
                    Some("js") => "text/javascript; charset=utf-8",
                    Some("wasm") => "application/wasm",
                    Some("json") => "application/json",
                    Some("png") => "image/png",
                    Some("svg") => "image/svg+xml",
                    Some("txt") => "text/plain; charset=utf-8",
                    _ => "application/octet-stream",
                };
                tiny_http::Response::from_data(bytes)
                    .with_header(
                        tiny_http::Header::from_bytes("Content-Type", mime).expect("header"),
                    )
                    .with_header(
                        tiny_http::Header::from_bytes("Cache-Control", "no-cache").expect("header"),
                    )
            }
            Err(_) => tiny_http::Response::from_string("not found").with_status_code(404),
        };
        let _ = request.respond(response);
    }
    Ok(())
}
