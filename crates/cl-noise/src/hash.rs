//! The prototype's hashes and noise, operation for operation.

use crate::js::{to_int32, ushr};
use crate::vec::V3;

/// `hash2(x, y)`: sine-based hash in `[0, 1)`. Used for the cliff strata, the surf shape and the
/// coast wall UV phase.
pub fn hash2(x: f64, y: f64) -> f64 {
    let s = libm::sin(x * 127.1 + y * 311.7) * 43758.5453;
    s - s.floor()
}

/// `hash3i(x, y, z)`: integer hash in `[0, 1)`; the workhorse behind every jitter and dither.
pub fn hash3i(x: i32, y: i32, z: i32) -> f64 {
    let mut n =
        x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263) ^ z.wrapping_mul(1_103_515_245);
    n = (n ^ (ushr(n, 13) as i32)).wrapping_mul(1_274_126_177);
    f64::from(ushr(n ^ (ushr(n, 16) as i32), 0)) / 4_294_967_296.0
}

/// [`hash3i`] on doubles, applying `ToInt32` to each argument as `Math.imul` does.
pub fn hash3(x: f64, y: f64, z: f64) -> f64 {
    hash3i(to_int32(x), to_int32(y), to_int32(z))
}

/// `vnoise3(x, y, z)`: smoothstep-interpolated value noise on the integer lattice.
pub fn vnoise3(x: f64, y: f64, z: f64) -> f64 {
    let xi = x.floor();
    let yi = y.floor();
    let zi = z.floor();
    let xf = x - xi;
    let yf = y - yi;
    let zf = z - zi;
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);
    let w = zf * zf * (3.0 - 2.0 * zf);
    let l = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let c = |i: f64, j: f64, k: f64| hash3(xi + i, yi + j, zi + k);
    l(
        l(
            l(c(0.0, 0.0, 0.0), c(1.0, 0.0, 0.0), u),
            l(c(0.0, 1.0, 0.0), c(1.0, 1.0, 0.0), u),
            v,
        ),
        l(
            l(c(0.0, 0.0, 1.0), c(1.0, 0.0, 1.0), u),
            l(c(0.0, 1.0, 1.0), c(1.0, 1.0, 1.0), u),
            v,
        ),
        w,
    )
}

/// Default octave count of [`fbm3`].
pub const FBM_OCT: u32 = 4;
/// Default base frequency of [`fbm3`].
pub const FBM_F0: f64 = 3.4;

/// `fbm3(p, seed, oct, f0)`: `oct` octaves of [`vnoise3`] with lacunarity 2.07 and gain 0.5,
/// normalised to `[0, 1]`. `oct == 0` and `f0 == 0.0` select the defaults, as `oct || FBM_OCT`
/// does in JavaScript. `seed` is a double because the prototype offsets it by fractions.
pub fn fbm3(p: V3, seed: f64, oct: u32, f0: f64) -> f64 {
    let oct = if oct == 0 { FBM_OCT } else { oct };
    let f0 = if f0 == 0.0 { FBM_F0 } else { f0 };
    let mut seed = seed;
    let mut s = 0.0;
    let mut a = 0.5;
    let mut f = f0;
    let mut norm = 0.0;
    for _ in 0..oct {
        s += a * vnoise3(
            p[0] * f + seed,
            p[1] * f + seed * 1.7,
            p[2] * f + seed * 2.3,
        );
        norm += a;
        f *= 2.07;
        a *= 0.5;
        seed += 11.0;
    }
    s / norm
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn outputs_stay_in_unit_interval() {
        for i in -50..50 {
            let h = hash3i(i, i * 7, -i);
            assert!((0.0..1.0).contains(&h));
            let n = vnoise3(f64::from(i) * 0.37, 1.5, -2.25);
            assert!((0.0..=1.0).contains(&n));
        }
        let f = fbm3([0.3, -0.4, 0.86], 31676.0, 0, 0.0);
        assert!((0.0..=1.0).contains(&f));
    }
}
