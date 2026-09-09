#![allow(clippy::pedantic, clippy::print_stderr, clippy::print_stdout)]
//! Replays `fixtures/noise/*.json`, extracted from the prototype by `reference/harness/extract.mjs`.
//! Integer hashes, value noise, fBm and mulberry32 are compared bit for bit. `sin`, `cos` and
//! `pow` are compared within 2 ulp until the fdlibm port lands (see the `cl-noise` issue
//! "bit-exact sin/cos/pow"): `libm` descends from FreeBSD's rewrite of fdlibm while V8 ships the
//! original, and the two disagree in the last bit for about 1 % of arguments.

use std::fs;
use std::path::PathBuf;

use cl_noise::js::{hypot2, hypot3, imul, round, to_fixed6, to_int32, ushr};
use cl_noise::{Mulberry32, fbm3, hash2, hash3, vnoise3};
use serde_json::Value;

/// Allowed distance, in units in the last place, for functions not yet ported from fdlibm.
const TRANSCENDENTAL_ULPS: u64 = 2;
/// `hash2` scales `sin` by 43758.5453 before taking the fraction, so 1 ulp of `sin` becomes ~1e-11.
const HASH2_TOLERANCE: f64 = 1e-9;

fn fixture(name: &str) -> Value {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "fixtures",
        "noise",
        name,
    ]
    .iter()
    .collect();
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).expect("valid fixture json")
}

/// Numbers, or the strings the harness writes for non-finite values.
fn num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().expect("f64"),
        Value::String(s) => match s.as_str() {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            other => panic!("unexpected numeric string {other:?}"),
        },
        other => panic!("not a number: {other}"),
    }
}

fn arr(v: &Value) -> Vec<f64> {
    v.as_array().expect("array").iter().map(num).collect()
}

fn ulps_apart(a: f64, b: f64) -> u64 {
    if a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan()) {
        return 0;
    }
    if a.is_nan() || b.is_nan() || a.signum() != b.signum() {
        return u64::MAX;
    }
    a.to_bits().abs_diff(b.to_bits())
}

/// Collects mismatches so one run reports them all.
#[derive(Default)]
struct Check {
    total: usize,
    failures: Vec<String>,
    /// Values that passed only thanks to a tolerance, by function name.
    inexact: Vec<(String, u64)>,
}

impl Check {
    fn bits(&mut self, what: impl FnOnce() -> String, expected: f64, got: f64) {
        self.total += 1;
        if ulps_apart(expected, got) != 0 {
            self.failures
                .push(format!("{}: expected {expected:e} got {got:e}", what()));
        }
    }

    fn ulps(&mut self, name: &str, what: impl FnOnce() -> String, expected: f64, got: f64) {
        self.total += 1;
        let d = ulps_apart(expected, got);
        if d > TRANSCENDENTAL_ULPS {
            self.failures.push(format!(
                "{}: expected {expected:e} got {got:e} ({d} ulp)",
                what()
            ));
        } else if d > 0 {
            self.inexact.push((name.to_owned(), d));
        }
    }

    fn within(&mut self, tol: f64, what: impl FnOnce() -> String, expected: f64, got: f64) {
        self.total += 1;
        let d = (expected - got).abs();
        if d > tol {
            self.failures.push(format!(
                "{}: expected {expected:e} got {got:e} (diff {d:e})",
                what()
            ));
        } else if d > 0.0 {
            self.inexact.push(("hash2".to_owned(), 1));
        }
    }

    fn eq<T: PartialEq + std::fmt::Debug>(
        &mut self,
        what: impl FnOnce() -> String,
        expected: T,
        got: T,
    ) {
        self.total += 1;
        if expected != got {
            self.failures
                .push(format!("{}: expected {expected:?} got {got:?}", what()));
        }
    }

    fn finish(self, name: &str) {
        assert!(self.total > 0, "{name}: fixture is empty");
        if !self.inexact.is_empty() {
            let mut names: Vec<&str> = self.inexact.iter().map(|(n, _)| n.as_str()).collect();
            names.sort_unstable();
            names.dedup();
            let summary: Vec<String> = names
                .iter()
                .map(|n| {
                    format!(
                        "{n}: {}",
                        self.inexact.iter().filter(|(m, _)| m == n).count()
                    )
                })
                .collect();
            eprintln!(
                "{name}: {} of {} values within tolerance but not exact ({})",
                self.inexact.len(),
                self.total,
                summary.join(", ")
            );
        }
        assert!(
            self.failures.is_empty(),
            "{name}: {} of {} values differ:\n{}",
            self.failures.len(),
            self.total,
            self.failures
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[test]
fn hash3i_matches_prototype() {
    let mut c = Check::default();
    for row in fixture("hash3i.json").as_array().unwrap() {
        let i = arr(&row["in"]);
        c.bits(
            || format!("hash3i{i:?}"),
            num(&row["out"]),
            hash3(i[0], i[1], i[2]),
        );
    }
    c.finish("hash3i");
}

#[test]
fn hash2_matches_prototype_within_sin_drift() {
    let mut c = Check::default();
    for row in fixture("hash2.json").as_array().unwrap() {
        let i = arr(&row["in"]);
        c.within(
            HASH2_TOLERANCE,
            || format!("hash2{i:?}"),
            num(&row["out"]),
            hash2(i[0], i[1]),
        );
    }
    c.finish("hash2");
}

#[test]
fn vnoise3_matches_prototype() {
    let mut c = Check::default();
    for row in fixture("vnoise3.json").as_array().unwrap() {
        let i = arr(&row["in"]);
        c.bits(
            || format!("vnoise3{i:?}"),
            num(&row["out"]),
            vnoise3(i[0], i[1], i[2]),
        );
    }
    c.finish("vnoise3");
}

#[test]
fn fbm3_matches_prototype() {
    let mut c = Check::default();
    for row in fixture("fbm3.json").as_array().unwrap() {
        let p = arr(&row["p"]);
        let seed = num(&row["seed"]);
        let oct = if row["oct"].is_null() {
            0
        } else {
            num(&row["oct"]) as u32
        };
        let f0 = if row["f0"].is_null() {
            0.0
        } else {
            num(&row["f0"])
        };
        c.bits(
            || format!("fbm3({p:?}, {seed}, {oct}, {f0})"),
            num(&row["out"]),
            fbm3([p[0], p[1], p[2]], seed, oct, f0),
        );
    }
    c.finish("fbm3");
}

#[test]
fn mulberry32_matches_prototype() {
    let mut c = Check::default();
    for row in fixture("mulberry32.json").as_array().unwrap() {
        let seed = num(&row["seed"]);
        let mut g = Mulberry32::new(seed);
        for (k, expected) in arr(&row["out"]).into_iter().enumerate() {
            c.bits(
                || format!("mulberry32({seed})[{k}]"),
                expected,
                g.next_f64(),
            );
        }
    }
    c.finish("mulberry32");
}

#[test]
fn js_semantics_match_v8() {
    let f = fixture("js-semantics.json");
    let mut c = Check::default();
    for row in f["toInt32"].as_array().unwrap() {
        let x = num(&row["in"]);
        c.eq(
            || format!("({x:e})|0"),
            num(&row["out"]) as i32,
            to_int32(x),
        );
    }
    for row in f["round"].as_array().unwrap() {
        let x = num(&row["in"]);
        c.bits(|| format!("Math.round({x:e})"), num(&row["out"]), round(x));
    }
    for row in f["toFixed6"].as_array().unwrap() {
        let x = num(&row["in"]);
        c.eq(
            || format!("({x:e}).toFixed(6)"),
            row["out"].as_str().unwrap().to_owned(),
            to_fixed6(x),
        );
    }
    for row in f["hypot3"].as_array().unwrap() {
        let i = arr(&row["in"]);
        c.bits(
            || format!("Math.hypot{i:?}"),
            num(&row["out"]),
            hypot3(i[0], i[1], i[2]),
        );
    }
    for row in f["hypot2"].as_array().unwrap() {
        let i = arr(&row["in"]);
        c.bits(
            || format!("Math.hypot{i:?}"),
            num(&row["out"]),
            hypot2(i[0], i[1]),
        );
    }
    for row in f["imul"].as_array().unwrap() {
        let i = arr(&row["in"]);
        c.eq(
            || format!("Math.imul{i:?}"),
            num(&row["out"]) as i32,
            imul(i[0] as i32, i[1] as i32),
        );
    }
    for row in f["ushr"].as_array().unwrap() {
        let i = arr(&row["in"]);
        c.eq(
            || format!("{}>>>{}", i[0], i[1]),
            num(&row["out"]) as u32,
            ushr(to_int32(i[0]), i[1] as u32),
        );
    }
    c.finish("js-semantics");
}

#[test]
fn libm_matches_v8_transcendentals() {
    let f = fixture("js-semantics.json");
    let mut c = Check::default();
    for row in f["trig"].as_array().unwrap() {
        let (x, y, u) = (num(&row["x"]), num(&row["y"]), num(&row["u"]));
        c.ulps(
            "sin",
            || format!("sin({x:e})"),
            num(&row["sin"]),
            libm::sin(x),
        );
        c.ulps(
            "cos",
            || format!("cos({x:e})"),
            num(&row["cos"]),
            libm::cos(x),
        );
        c.bits(
            || format!("atan2({y:e},{x:e})"),
            num(&row["atan2"]),
            libm::atan2(y, x),
        );
        c.bits(|| format!("acos({y:e})"), num(&row["acos"]), libm::acos(y));
        c.ulps(
            "pow",
            || format!("pow({u:e},1.7)"),
            num(&row["pow17"]),
            libm::pow(u, 1.7),
        );
        c.ulps(
            "pow",
            || format!("pow({u:e},2.2)"),
            num(&row["pow22"]),
            libm::pow(u, 2.2),
        );
        c.ulps(
            "pow",
            || format!("pow({u:e},1.6)"),
            num(&row["pow16"]),
            libm::pow(u, 1.6),
        );
        c.bits(|| format!("sqrt({u:e})"), num(&row["sqrt"]), u.sqrt());
    }
    for row in f["sinHash"].as_array().unwrap() {
        let x = num(&row["x"]);
        c.ulps(
            "sin",
            || format!("sin({x:e})"),
            num(&row["sin"]),
            libm::sin(x),
        );
    }
    c.finish("transcendentals");
}
