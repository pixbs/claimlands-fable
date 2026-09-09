//! Repository lints that cargo cannot express: the crate dependency graph, prose padding, the
//! TODO policy, banned words in tracked files, and the required crate README sections.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use regex::{Regex, RegexBuilder};
use walkdir::WalkDir;
use xshell::{Shell, cmd};

use crate::root;

/// Allowed internal dependencies per crate (`docs/architecture.md`). A crate may depend only on
/// what is listed; core crates never see rendering, windowing or UI crates.
const ALLOWED: &[(&str, &[&str])] = &[
    ("cl-model", &[]),
    ("cl-noise", &[]),
    ("cl-hexsphere", &["cl-model", "cl-noise"]),
    ("cl-worldgen", &["cl-model", "cl-hexsphere", "cl-noise"]),
    ("cl-pixelart", &["cl-model", "cl-hexsphere", "cl-noise"]),
    (
        "cl-scenery",
        &["cl-model", "cl-hexsphere", "cl-noise", "cl-pixelart"],
    ),
    ("cl-rules", &["cl-model", "cl-noise"]),
    ("cl-level", &["cl-model", "cl-hexsphere", "cl-worldgen"]),
    ("cl-ai", &["cl-model", "cl-rules", "cl-noise"]),
    (
        "cl-session",
        &["cl-model", "cl-rules", "cl-ai", "cl-level", "cl-hexsphere"],
    ),
    ("cl-render", &["cl-model", "cl-noise"]),
    ("cl-ui", &["cl-model"]),
    (
        "cl-app",
        &[
            "cl-model",
            "cl-hexsphere",
            "cl-worldgen",
            "cl-pixelart",
            "cl-scenery",
            "cl-rules",
            "cl-level",
            "cl-ai",
            "cl-session",
            "cl-render",
            "cl-ui",
            "cl-noise",
        ],
    ),
    ("xtask", &[]),
];

/// External crates the engine-free core must never pull in.
const CORE_BANNED: &[&str] = &[
    "wgpu",
    "winit",
    "egui",
    "web-sys",
    "rand",
    "tokio",
    "getrandom",
];
const CORE_CRATES: &[&str] = &[
    "cl-model",
    "cl-noise",
    "cl-hexsphere",
    "cl-worldgen",
    "cl-pixelart",
    "cl-scenery",
    "cl-rules",
    "cl-level",
    "cl-ai",
    "cl-session",
];

/// Phrases that add words without facts, as case-insensitive regular expressions.
const FILLER: &[&str] = &[
    r"\bit is important to note\b",
    r"\bit's important to note\b",
    r"\bit is worth noting\b",
    r"\bas mentioned above\b",
    r"\bas we can see\b",
    r"\bin order to\b",
    r"\bcomprehensive\b",
    r"\brobust\b",
    r"\bleverage\b",
    r"\bseamless(ly)?\b",
    r"\bdelve\b",
    r"\bcutting-edge\b",
    r"\bstate-of-the-art\b",
    r"\bin this document\b",
    r"\bthis document (describes|explains|provides)\b",
    r"\bthis file contains\b",
    r"\bplease note that\b",
    r"\bbasically\b",
    r"\bvery\b",
];

const README_SECTIONS: &[&str] = &[
    "## Purpose",
    "## Public API",
    "## Invariants",
    "## Testing",
    "## Non-goals",
];
const ADR_SECTIONS: &[&str] = &[
    "## Context",
    "## Decision",
    "## Consequences",
    "## Alternatives",
];

/// Runs every lint and reports all findings before failing.
pub fn run(sh: &Shell) -> Result<()> {
    println!("== lint-repo");
    let mut findings = Vec::new();
    findings.extend(arch(sh)?);
    findings.extend(prose()?);
    findings.extend(todo(sh)?);
    findings.extend(banned(sh)?);
    if findings.is_empty() {
        println!("lint-repo: clean");
        return Ok(());
    }
    for f in &findings {
        println!("  {f}");
    }
    bail!("lint-repo: {} finding(s)", findings.len());
}

/// Dependency direction between workspace crates, plus the engine-free rule for core crates.
fn arch(sh: &Shell) -> Result<Vec<String>> {
    let json = cmd!(sh, "cargo metadata --format-version 1 --no-deps").read()?;
    let meta: serde_json::Value = serde_json::from_str(&json)?;
    let allowed: BTreeMap<&str, &[&str]> = ALLOWED.iter().copied().collect();
    let mut out = Vec::new();
    for pkg in meta["packages"].as_array().context("packages")? {
        let name = pkg["name"].as_str().unwrap_or_default();
        let Some(list) = allowed.get(name) else {
            out.push(format!("arch: crate {name} is not in the allow-list in xtask/src/lint.rs and docs/architecture.md"));
            continue;
        };
        for dep in pkg["dependencies"].as_array().context("dependencies")? {
            let dep_name = dep["name"].as_str().unwrap_or_default();
            let is_dev = dep["kind"].as_str() == Some("dev");
            if dep_name.starts_with("cl-") && !list.contains(&dep_name) {
                out.push(format!(
                    "arch: {name} depends on {dep_name}, which the allow-list forbids"
                ));
            }
            if !is_dev && CORE_CRATES.contains(&name) && CORE_BANNED.contains(&dep_name) {
                out.push(format!("arch: core crate {name} depends on {dep_name}"));
            }
        }
    }
    Ok(out)
}

fn markdown_files() -> Vec<std::path::PathBuf> {
    let root = root();
    let mut files = Vec::new();
    for entry in WalkDir::new(&root).into_iter().filter_entry(|e| {
        let n = e.file_name().to_string_lossy();
        !(n == "target"
            || n == "node_modules"
            || n == "dist"
            || n == "vendor"
            || n == ".git"
            || n == "fixtures"
            || n == "reference")
    }) {
        let entry = entry.expect("walk");
        if entry.path().extension().is_some_and(|e| e == "md") {
            files.push(entry.into_path());
        }
    }
    files.sort();
    files
}

/// Filler phrases, empty sections, and required sections of crate READMEs and ADRs.
fn prose() -> Result<Vec<String>> {
    let root = root();
    let mut out = Vec::new();
    let filler: Vec<Regex> = FILLER
        .iter()
        .map(|f| {
            RegexBuilder::new(f)
                .case_insensitive(true)
                .build()
                .expect("regex")
        })
        .collect();
    for path in markdown_files() {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path).with_context(|| format!("read {rel}"))?;
        for (i, line) in text.lines().enumerate() {
            if line.trim_start().starts_with('|') || line.trim_start().starts_with("```") {
                continue;
            }
            for (f, re) in FILLER.iter().zip(&filler) {
                if re.is_match(line) {
                    out.push(format!("prose: {rel}:{}: filler {f:?}", i + 1));
                }
            }
        }
        for (i, line) in text.lines().enumerate() {
            if line.starts_with("## ") {
                let rest: Vec<&str> = text
                    .lines()
                    .skip(i + 1)
                    .take_while(|l| !l.starts_with("## "))
                    .collect();
                if rest.iter().all(|l| l.trim().is_empty()) {
                    out.push(format!(
                        "prose: {rel}:{}: section {:?} is empty",
                        i + 1,
                        line.trim()
                    ));
                }
            }
        }
        let is_crate_readme = rel.starts_with("crates/") && rel.ends_with("/README.md");
        let is_adr = rel.starts_with("docs/adr/")
            && !rel.ends_with("README.md")
            && !rel.contains("template");
        let required: &[&str] = if is_crate_readme {
            README_SECTIONS
        } else if is_adr {
            ADR_SECTIONS
        } else {
            &[]
        };
        for section in required {
            if !text.lines().any(|l| l.trim() == *section) {
                out.push(format!("prose: {rel}: missing section {section:?}"));
            }
        }
    }
    for entry in fs::read_dir(root.join("crates"))? {
        let entry = entry?;
        if entry.path().is_dir() && !entry.path().join("README.md").exists() {
            out.push(format!(
                "prose: crates/{}: README.md is missing",
                entry.file_name().to_string_lossy()
            ));
        }
    }
    Ok(out)
}

/// `TODO`/`FIXME`/`XXX` in code must reference an issue: `TODO(#123)`. Prose is not scanned, so
/// documentation may describe the rule.
fn todo(sh: &Shell) -> Result<Vec<String>> {
    let files = cmd!(sh, "git ls-files").read()?;
    let re = Regex::new(r"\b(TODO|FIXME|XXX)\b(\(#\d+\))?")?;
    let mut out = Vec::new();
    for rel in files.lines() {
        if !has_ext(rel, &["rs", "wgsl", "yml", "mjs", "toml", "sh", "kts"])
            || rel.starts_with("vendor/")
            || rel.starts_with("reference/")
        {
            continue;
        }
        let Ok(text) = fs::read_to_string(root().join(rel)) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            for cap in re.captures_iter(line) {
                if cap.get(2).is_none() && !rel.ends_with("lint.rs") {
                    out.push(format!(
                        "todo: {rel}:{}: {} without an issue reference, write {}(#<issue>)",
                        i + 1,
                        &cap[1],
                        &cap[1]
                    ));
                }
            }
        }
    }
    Ok(out)
}

/// Words that must not appear in tracked files (`.github/policy/banned.txt`), outside the files
/// that exist to name them.
fn banned(sh: &Shell) -> Result<Vec<String>> {
    let policy = root().join(".github/policy/banned.txt");
    let Ok(text) = fs::read_to_string(&policy) else {
        return Ok(vec![]);
    };
    let patterns: Vec<Regex> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            RegexBuilder::new(l)
                .case_insensitive(true)
                .build()
                .with_context(|| format!("banned pattern {l:?}"))
        })
        .collect::<Result<_>>()?;
    let exempt = |rel: &str| {
        rel.starts_with(".claude/")
            || rel == "CLAUDE.md"
            || rel.starts_with(".github/policy/")
            || rel.starts_with(".githooks/")
            || rel.starts_with("vendor/")
            || rel.starts_with("reference/")
            || rel.starts_with("fixtures/")
            || has_ext(rel, &["png"])
            || rel == "xtask/src/lint.rs"
            || rel == "Cargo.lock"
    };
    // The configuration directory and file of one agent tool are facts a document may state.
    let path_tokens = Regex::new(r"\.claude/\S*|CLAUDE\.md")?;
    let files = cmd!(sh, "git ls-files").read()?;
    let mut out = Vec::new();
    for rel in files.lines().filter(|r| !exempt(r)) {
        let Ok(text) = fs::read_to_string(root().join(rel)) else {
            continue;
        };
        for (i, raw) in text.lines().enumerate() {
            let line = path_tokens.replace_all(raw, "");
            for re in &patterns {
                if re.is_match(&line) {
                    out.push(format!(
                        "banned: {rel}:{}: matches {:?}",
                        i + 1,
                        re.as_str()
                    ));
                }
            }
        }
    }
    Ok(out)
}

/// Whether `rel` has one of `exts` as its extension, case-insensitively.
fn has_ext(rel: &str, exts: &[&str]) -> bool {
    Path::new(rel)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
}
