//! JavaScript number semantics the prototype depends on. Each function reproduces the ECMAScript
//! operation (or V8's implementation of it) exactly, so ports can keep the prototype's expressions.

/// `ToInt32(x)`: what `x | 0` and the operands of `Math.imul`, `^` and `>>>` become.
pub fn to_int32(x: f64) -> i32 {
    if !x.is_finite() {
        return 0;
    }
    let t = x.trunc();
    let m = t % 4_294_967_296.0;
    let m = if m < 0.0 { m + 4_294_967_296.0 } else { m };
    (m as u32) as i32
}

/// `Math.imul(a, b)`.
pub fn imul(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b)
}

/// `x >>> n` for an int32 `x`; the result is an unsigned 32-bit value.
pub fn ushr(x: i32, n: u32) -> u32 {
    (x as u32) >> (n & 31)
}

/// `Math.round(x)`: nearest integer with ties toward +∞. Differs from `f64::round` (ties away
/// from zero) for negative halves, and from `floor(x + 0.5)` for `0.49999999999999994`.
pub fn round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let f = x.floor();
    if x - f >= 0.5 { f + 1.0 } else { f }
}

/// `Math.floor(x + 0.5)`: the quantiser the fixture hashes use (`hashQ6` in the harness).
pub fn floor_half_up(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// `Math.hypot(...values)` as V8 computes it: every value is scaled by the largest magnitude, the
/// squares are summed with Kahan compensation, and the root is scaled back. A plain
/// `sqrt(x² + y² + z²)` differs in the last bit often enough to move dithered texels.
///
/// # Panics
/// With more than eight values (the prototype never passes more than three).
pub fn hypot(values: &[f64]) -> f64 {
    assert!(values.len() <= 8, "hypot supports up to 8 values");
    let mut abs = [0.0f64; 8];
    let mut max = 0.0f64;
    let mut one_nan = false;
    for (i, &v) in values.iter().enumerate() {
        if v.is_nan() {
            one_nan = true;
        } else {
            let a = v.abs();
            abs[i] = a;
            if a > max {
                max = a;
            }
        }
    }
    if max == f64::INFINITY {
        return f64::INFINITY;
    }
    if one_nan {
        return f64::NAN;
    }
    if max == 0.0 {
        return 0.0;
    }
    let mut sum = 0.0f64;
    let mut compensation = 0.0f64;
    for &a in &abs[..values.len()] {
        let n = a / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}

/// `Math.hypot(x, y)`.
pub fn hypot2(x: f64, y: f64) -> f64 {
    hypot(&[x, y])
}

/// `Math.hypot(x, y, z)`.
pub fn hypot3(x: f64, y: f64, z: f64) -> f64 {
    hypot(&[x, y, z])
}

/// `x.toFixed(6)` for `|x| < 1e21`, computed exactly from the binary value: the integer `n`
/// minimising `|n / 10⁶ − |x||`, ties to the larger `n`, with a `-` sign for any negative input
/// (so `-1e-10` gives `-0.000000`, and `-0.0` gives `0.000000`, both as in JavaScript).
pub fn to_fixed6(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_owned();
    }
    if x < 0.0 {
        return format!("-{}", fixed6_abs(-x));
    }
    fixed6_abs(x)
}

fn fixed6_abs(x: f64) -> String {
    debug_assert!(
        (0.0..1e21).contains(&x),
        "toFixed(6) port covers |x| < 1e21"
    );
    let bits = x.to_bits();
    let exp_bits = ((bits >> 52) & 0x7ff) as i32;
    let frac = bits & ((1u64 << 52) - 1);
    let (mant, exp) = if exp_bits == 0 {
        (frac, -1074)
    } else {
        (frac | (1u64 << 52), exp_bits - 1075)
    };
    let scaled = u128::from(mant) * 1_000_000u128;
    let n: u128 = if exp >= 0 {
        scaled << exp
    } else {
        let sh = (-exp) as u32;
        if sh >= 128 {
            0
        } else {
            let q = scaled >> sh;
            let r = scaled & ((1u128 << sh) - 1);
            if r >= (1u128 << (sh - 1)) { q + 1 } else { q }
        }
    };
    format!("{}.{:06}", n / 1_000_000, n % 1_000_000)
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn to_int32_wraps_like_the_spec() {
        assert_eq!(to_int32(3.7), 3);
        assert_eq!(to_int32(-3.7), -3);
        assert_eq!(to_int32(4_294_967_297.0), 1);
        assert_eq!(to_int32(2_147_483_648.0), i32::MIN);
        assert_eq!(to_int32(f64::NAN), 0);
        assert_eq!(to_int32(f64::INFINITY), 0);
    }

    #[test]
    fn round_ties_toward_positive_infinity() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(0.499_999_999_999_999_94), 0.0);
        assert_eq!(round(-0.5), 0.0);
    }

    #[test]
    fn to_fixed6_matches_javascript_edge_cases() {
        assert_eq!(to_fixed6(0.5), "0.500000");
        assert_eq!(to_fixed6(-0.0), "0.000000");
        assert_eq!(to_fixed6(-1e-10), "-0.000000");
        assert_eq!(to_fixed6(1.000_000_5), "1.000001");
        assert_eq!(to_fixed6(123_456.123_456_789), "123456.123457");
    }

    #[test]
    fn hypot_handles_specials() {
        assert_eq!(hypot3(0.0, 0.0, 0.0), 0.0);
        assert_eq!(hypot3(3.0, 4.0, 0.0), 5.0);
        assert!(hypot3(f64::NAN, 1.0, 2.0).is_nan());
        assert_eq!(hypot3(f64::INFINITY, 1.0, f64::NAN), f64::INFINITY);
    }
}
