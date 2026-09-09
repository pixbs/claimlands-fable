//! Picking work: issues that are open, `ready`, unassigned and unblocked, ordered by milestone
//! and priority; claiming one; creating its worktree and branch.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use xshell::{Shell, cmd};

use crate::root;

struct Issue {
    number: u64,
    title: String,
    url: String,
    milestone: Option<(u64, String)>,
    prio: u8,
    kind: String,
    assigned: bool,
    blocked_by_open: Vec<u64>,
}

fn repo(sh: &Shell) -> Result<String> {
    Ok(
        cmd!(sh, "gh repo view --json nameWithOwner -q .nameWithOwner")
            .read()?
            .trim()
            .to_owned(),
    )
}

fn load(sh: &Shell) -> Result<Vec<Issue>> {
    let repo = repo(sh)?;
    let json = cmd!(sh, "gh issue list --state open --label ready --limit 200 --json number,title,url,assignees,milestone,labels").read()?;
    let list: Vec<Value> = serde_json::from_str(&json)?;
    let mut issues = Vec::new();
    for v in list {
        let number = v["number"].as_u64().context("number")?;
        let labels: Vec<String> = v["labels"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|l| l["name"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        let prio = labels
            .iter()
            .find_map(|l| l.strip_prefix("prio:p"))
            .and_then(|p| p.parse().ok())
            .unwrap_or(3);
        let kind = labels
            .iter()
            .find_map(|l| l.strip_prefix("type:"))
            .unwrap_or("task")
            .to_owned();
        let endpoint = format!("repos/{repo}/issues/{number}/dependencies/blocked_by");
        let deps = cmd!(sh, "gh api -H X-GitHub-Api-Version:2026-03-10 {endpoint}")
            .ignore_status()
            .ignore_stderr()
            .read()
            .unwrap_or_else(|_| "[]".into());
        let deps: Vec<Value> = serde_json::from_str(&deps).unwrap_or_default();
        let blocked_by_open = deps
            .iter()
            .filter(|d| d["state"].as_str() == Some("open"))
            .filter_map(|d| d["number"].as_u64())
            .collect();
        issues.push(Issue {
            number,
            title: v["title"].as_str().unwrap_or_default().to_owned(),
            url: v["url"].as_str().unwrap_or_default().to_owned(),
            milestone: v["milestone"]["number"].as_u64().map(|n| {
                (
                    n,
                    v["milestone"]["title"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                )
            }),
            prio,
            kind,
            assigned: v["assignees"].as_array().is_some_and(|a| !a.is_empty()),
            blocked_by_open,
        });
    }
    issues.sort_by_key(|i| {
        (
            i.milestone.as_ref().map_or(u64::MAX, |m| m.0),
            i.prio,
            i.number,
        )
    });
    Ok(issues)
}

fn available(issues: &[Issue]) -> Vec<&Issue> {
    let first_milestone = issues
        .iter()
        .filter(|i| !i.assigned && i.blocked_by_open.is_empty())
        .find_map(|i| i.milestone.as_ref().map(|m| m.0));
    issues
        .iter()
        .filter(|i| !i.assigned && i.blocked_by_open.is_empty())
        .filter(|i| first_milestone.is_none_or(|m| i.milestone.as_ref().is_none_or(|im| im.0 == m)))
        .collect()
}

/// Prints the available issues and, below, what is blocked or claimed.
pub fn next(sh: &Shell) -> Result<()> {
    let issues = load(sh)?;
    let open = available(&issues);
    if open.is_empty() {
        println!("no available issues (open + ready + unassigned + unblocked)");
    } else {
        println!("available now (earliest milestone first):");
        for i in &open {
            println!(
                "  #{:<4} p{} {:<5} {:<14} {}\n        {}",
                i.number,
                i.prio,
                i.kind,
                i.milestone.as_ref().map_or("-", |m| m.1.as_str()),
                i.title,
                i.url
            );
        }
    }
    let waiting: Vec<&Issue> = issues
        .iter()
        .filter(|i| i.assigned || !i.blocked_by_open.is_empty())
        .collect();
    if !waiting.is_empty() {
        println!("not available:");
        for i in waiting {
            let why = if i.assigned {
                "claimed".to_owned()
            } else {
                format!(
                    "blocked by {}",
                    i.blocked_by_open
                        .iter()
                        .map(|n| format!("#{n}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            println!("  #{:<4} {} ({why})", i.number, i.title);
        }
    }
    Ok(())
}

/// Claims an issue by assigning yourself, then creates the worktree and branch.
pub fn take(sh: &Shell, number: Option<u64>) -> Result<()> {
    let issues = load(sh)?;
    let issue = match number {
        Some(n) => issues
            .iter()
            .find(|i| i.number == n)
            .with_context(|| format!("#{n} is not an open ready issue"))?,
        None => *available(&issues)
            .first()
            .context("no available issue; run cargo xtask next")?,
    };
    if issue.assigned {
        bail!("#{} is already claimed", issue.number);
    }
    if !issue.blocked_by_open.is_empty() {
        bail!(
            "#{} is blocked by {:?}",
            issue.number,
            issue.blocked_by_open
        );
    }
    let n = issue.number.to_string();
    cmd!(sh, "gh issue edit {n} --add-assignee @me").run()?;
    println!("claimed #{}: {}", issue.number, issue.title);
    let slug = slug_of(&issue.title);
    worktree_for(sh, issue.number, &issue.kind, &slug)?;
    let body = cmd!(sh, "gh issue view {n} --json body -q .body")
        .read()
        .unwrap_or_default();
    println!("\n--- issue body ---\n{body}");
    Ok(())
}

/// Creates the worktree and branch for an issue without touching GitHub assignment.
pub fn worktree(sh: &Shell, number: u64, slug: Option<String>) -> Result<()> {
    let n = number.to_string();
    let json = cmd!(sh, "gh issue view {n} --json title,labels").read()?;
    let v: Value = serde_json::from_str(&json)?;
    let kind = v["labels"]
        .as_array()
        .and_then(|a| {
            a.iter()
                .find_map(|l| l["name"].as_str().and_then(|s| s.strip_prefix("type:")))
        })
        .unwrap_or("task")
        .to_owned();
    let slug = slug.unwrap_or_else(|| slug_of(v["title"].as_str().unwrap_or("work")));
    worktree_for(sh, number, &kind, &slug)
}

fn worktree_for(sh: &Shell, number: u64, kind: &str, slug: &str) -> Result<()> {
    let prefix = match kind {
        "feat" => "feat",
        "bug" => "fix",
        "docs" => "docs",
        "chore" => "chore",
        _ => "task",
    };
    let branch = format!("{prefix}/{number}-{slug}");
    let dir = root()
        .parent()
        .context("parent dir")?
        .join("claimlands-wt")
        .join(number.to_string());
    if dir.exists() {
        println!("worktree already exists: {}", dir.display());
        return Ok(());
    }
    cmd!(sh, "git fetch origin main").ignore_status().run()?;
    let base = if cmd!(sh, "git rev-parse --verify origin/main")
        .ignore_status()
        .ignore_stderr()
        .read()
        .is_ok()
    {
        "origin/main"
    } else {
        "main"
    };
    cmd!(sh, "git worktree add -b {branch} {dir} {base}").run()?;
    println!("worktree {} on branch {branch}", dir.display());
    println!("next: cd \"{}\" && cargo xtask setup", dir.display());
    Ok(())
}

/// Lowercase, ASCII letters and digits, dashes between words, at most 40 characters.
pub fn slug_of(title: &str) -> String {
    let mut s = String::new();
    let mut dash = false;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
            dash = false;
        } else if !dash && !s.is_empty() {
            s.push('-');
            dash = true;
        }
        if s.len() >= 40 {
            break;
        }
    }
    s.trim_end_matches('-').to_owned()
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::slug_of;

    #[test]
    fn slugs_are_branch_safe() {
        assert_eq!(
            slug_of("Port ground atlas (section 3b)"),
            "port-ground-atlas-section-3b"
        );
        assert_eq!(slug_of("  weird -- title!! "), "weird-title");
        assert!(slug_of(&"x".repeat(100)).len() <= 40);
    }
}
