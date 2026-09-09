//! `Math.sin`, `Math.cos` and `Math.pow` as V8 computes them.
//!
//! V8 evaluates the three in `src/base/ieee754.cc`, a translation of Sun's fdlibm. The `libm`
//! crate descends from FreeBSD's rewrite of the same code, which reassociates a few polynomial
//! sums and drops V8's parenthesisation in `pow`; the two land on a different last bit for about
//! 1 % of arguments, which is enough to move a dithered texel or shift a cliff stratum. This
//! module reproduces V8's version operation for operation, so `sin`, `cos` and `pow` are bit-exact
//! against the prototype on every platform.
//!
//! Adapted from fdlibm (<http://www.netlib.org/fdlibm>):
//!
//! > Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//! >
//! > Developed at SunSoft, a Sun Microsystems, Inc. business.
//! > Permission to use, copy, modify, and distribute this software is freely granted, provided
//! > that this notice is preserved.

// The decimal literals are fdlibm's own: each one is written so that the compiler rounds it to the
// bit pattern named in the C comment, and shortening them would break the line-by-line comparison
// with the original.
#![allow(clippy::excessive_precision)]
// fdlibm builds its NaNs as `x - x`, which also raises the invalid-operation flag for a signalling
// argument; `f64::NAN` would not.
#![allow(clippy::eq_op)]
// `INVPIO2`, `LG2` and `IVLN2` are near 2/pi, ln 2 and log2 e, but each comes with the split halves
// fdlibm pairs it with (`PIO2_1`, `LG2_H`, `IVLN2_H`); swapping in a std constant would break the
// pairing the algorithm depends on.
#![allow(clippy::approx_constant)]

/// High word of `x` as fdlibm's signed `int32_t`.
fn hi(x: f64) -> i32 {
    (x.to_bits() >> 32) as u32 as i32
}

/// Low word of `x`.
fn lo(x: f64) -> u32 {
    x.to_bits() as u32
}

/// `x` with its high word replaced (fdlibm's `SET_HIGH_WORD`).
fn with_hi(x: f64, high: i32) -> f64 {
    words(high, lo(x))
}

/// `x` with its low word replaced (fdlibm's `SET_LOW_WORD`).
fn with_lo(x: f64, low: u32) -> f64 {
    words(hi(x), low)
}

/// The double built from a high and a low word (fdlibm's `INSERT_WORDS`).
fn words(high: i32, low: u32) -> f64 {
    f64::from_bits((u64::from(high as u32) << 32) | u64::from(low))
}

// ---------------------------------------------------------------- kernels

const S1: f64 = -1.66666666666666324348e-01;
const S2: f64 = 8.33333333332248946124e-03;
const S3: f64 = -1.98412698298579493134e-04;
const S4: f64 = 2.75573137070700676789e-06;
const S5: f64 = -2.50507602534068634195e-08;
const S6: f64 = 1.58969099521155010221e-10;

/// `__kernel_sin(x, y, iy)`: sine on `[-π/4, π/4]`, with `y` the tail of `x` and `iy == 0`
/// meaning the tail is zero.
fn kernel_sin(x: f64, y: f64, iy: i32) -> f64 {
    let ix = hi(x) & 0x7fff_ffff;
    if ix < 0x3e40_0000 && x as i32 == 0 {
        /* |x| < 2^-27 */
        return x;
    }
    let z = x * x;
    let v = z * x;
    let r = S2 + z * (S3 + z * (S4 + z * (S5 + z * S6)));
    if iy == 0 {
        x + v * (S1 + z * r)
    } else {
        x - ((z * (0.5 * y - v * r) - y) - v * S1)
    }
}

const C1: f64 = 4.16666666666666019037e-02;
const C2: f64 = -1.38888888888741095749e-03;
const C3: f64 = 2.48015872894767294178e-05;
const C4: f64 = -2.75573143513906633035e-07;
const C5: f64 = 2.08757232129817482790e-09;
const C6: f64 = -1.13596475577881948265e-11;

/// `__kernel_cos(x, y)`: cosine on `[-π/4, π/4]`, with `y` the tail of `x`.
fn kernel_cos(x: f64, y: f64) -> f64 {
    let ix = hi(x) & 0x7fff_ffff;
    if ix < 0x3e40_0000 && x as i32 == 0 {
        /* |x| < 2^-27 */
        return 1.0;
    }
    let z = x * x;
    let r = z * (C1 + z * (C2 + z * (C3 + z * (C4 + z * (C5 + z * C6)))));
    if ix < 0x3fd3_3333 {
        /* |x| < 0.3 */
        1.0 - (0.5 * z - (z * r - x * y))
    } else {
        let qx = if ix > 0x3fe9_0000 {
            /* x > 0.78125 */
            0.28125
        } else {
            words(ix - 0x0020_0000, 0) /* x/4 */
        };
        let iz = 0.5 * z - qx;
        let a = 1.0 - qx;
        a - (iz - (z * r - x * y))
    }
}

// ---------------------------------------------------------------- argument reduction

/// 396 hex digits of 2/π, 24 bits per entry.
const TWO_OVER_PI: [i32; 66] = [
    0xA2F983, 0x6E4E44, 0x1529FC, 0x2757D1, 0xF534DD, 0xC0DB62, 0x95993C, 0x439041, 0xFE5163,
    0xABDEBB, 0xC561B7, 0x246E3A, 0x424DD2, 0xE00649, 0x2EEA09, 0xD1921C, 0xFE1DEB, 0x1CB129,
    0xA73EE8, 0x8235F5, 0x2EBB44, 0x84E99C, 0x7026B4, 0x5F7E41, 0x3991D6, 0x398353, 0x39F49C,
    0x845F8B, 0xBDF928, 0x3B1FF8, 0x97FFDE, 0x05980F, 0xEF2F11, 0x8B5A0A, 0x6D1F6D, 0x367ECF,
    0x27CB09, 0xB74F46, 0x3F669E, 0x5FEA2D, 0x7527BA, 0xC7EBE5, 0xF17B3D, 0x0739F7, 0x8A5292,
    0xEA6BFB, 0x5FB11F, 0x8D5D08, 0x560330, 0x46FC7B, 0x6BABF0, 0xCFBC20, 0x9AF436, 0x1DA9E3,
    0x91615E, 0xE61B08, 0x659985, 0x5F14A0, 0x68408D, 0xFFD880, 0x4D7327, 0x310606, 0x1556CA,
    0x73A8C9, 0x60E27B, 0xC08C6B,
];

/// High words of `n * π/2` for `n` in `1..=32`; the quick "no cancellation" check.
const NPIO2_HW: [i32; 32] = [
    0x3FF921FB, 0x400921FB, 0x4012D97C, 0x401921FB, 0x401F6A7A, 0x4022D97C, 0x4025FDBB, 0x402921FB,
    0x402C463A, 0x402F6A7A, 0x4031475C, 0x4032D97C, 0x40346B9C, 0x4035FDBB, 0x40378FDB, 0x403921FB,
    0x403AB41B, 0x403C463A, 0x403DD85A, 0x403F6A7A, 0x40407E4C, 0x4041475C, 0x4042106C, 0x4042D97C,
    0x4043A28C, 0x40446B9C, 0x404534AC, 0x4045FDBB, 0x4046C6CB, 0x40478FDB, 0x404858EB, 0x404921FB,
];

const TWO24: f64 = 1.67772160000000000000e+07;
const TWON24: f64 = 5.96046447753906250000e-08;
const INVPIO2: f64 = 6.36619772367581382433e-01;
const PIO2_1: f64 = 1.57079632673412561417e+00;
const PIO2_1T: f64 = 6.07710050650619224932e-11;
const PIO2_2: f64 = 6.07710050630396597660e-11;
const PIO2_2T: f64 = 2.02226624879595063154e-21;
const PIO2_3: f64 = 2.02226624871116645580e-21;
const PIO2_3T: f64 = 8.47842766036889956997e-32;

/// `__ieee754_rem_pio2(x, y)`: `x` reduced modulo π/2 into `y0 + y1`, plus the quadrant count
/// whose low two bits pick sine or cosine.
fn rem_pio2(x: f64) -> (i32, f64, f64) {
    let hx = hi(x);
    let ix = hx & 0x7fff_ffff;
    if ix <= 0x3fe9_21fb {
        /* |x| ~<= π/4, no need for reduction */
        return (0, x, 0.0);
    }
    if ix < 0x4002_d97c {
        /* |x| < 3π/4, special case with n = ±1 */
        if hx > 0 {
            let mut z = x - PIO2_1;
            if ix != 0x3ff9_21fb {
                /* 33+53 bit π is good enough */
                let y0 = z - PIO2_1T;
                return (1, y0, (z - y0) - PIO2_1T);
            }
            /* near π/2, use 33+33+53 bit π */
            z -= PIO2_2;
            let y0 = z - PIO2_2T;
            return (1, y0, (z - y0) - PIO2_2T);
        }
        let mut z = x + PIO2_1;
        if ix != 0x3ff9_21fb {
            let y0 = z + PIO2_1T;
            return (-1, y0, (z - y0) + PIO2_1T);
        }
        z += PIO2_2;
        let y0 = z + PIO2_2T;
        return (-1, y0, (z - y0) + PIO2_2T);
    }
    if ix <= 0x4139_21fb {
        /* |x| ~<= 2^19 (π/2), medium size */
        let mut t = x.abs();
        let n = (t * INVPIO2 + 0.5) as i32;
        let fn_ = f64::from(n);
        let mut r = t - fn_ * PIO2_1;
        let mut w = fn_ * PIO2_1T; /* 1st round good to 85 bit */
        let mut y0 = r - w; /* both fdlibm branches start here */
        if n >= 32 || ix == NPIO2_HW[(n - 1) as usize] {
            /* cancellation possible: refine until y0 keeps its bits */
            let j = ix >> 20;
            let mut i = j - ((hi(y0) >> 20) & 0x7ff);
            if i > 16 {
                /* 2nd iteration needed, good to 118 */
                t = r;
                w = fn_ * PIO2_2;
                r = t - w;
                w = fn_ * PIO2_2T - ((t - r) - w);
                y0 = r - w;
                i = j - ((hi(y0) >> 20) & 0x7ff);
                if i > 49 {
                    /* 3rd iteration needed, 151 bits of accuracy; covers all remaining cases */
                    t = r;
                    w = fn_ * PIO2_3;
                    r = t - w;
                    w = fn_ * PIO2_3T - ((t - r) - w);
                    y0 = r - w;
                }
            }
        }
        let y1 = (r - y0) - w;
        return if hx < 0 { (-n, -y0, -y1) } else { (n, y0, y1) };
    }
    if ix >= 0x7ff0_0000 {
        /* x is inf or NaN */
        return (0, x - x, x - x);
    }
    /* all other (large) arguments: z = scalbn(|x|, ilogb(x) - 23), split into 24-bit pieces */
    let e0 = (ix >> 20) - 1046;
    let mut z = words(ix - ((e0 as u32) << 20) as i32, lo(x));
    let mut tx = [0.0f64; 3];
    for t in tx.iter_mut().take(2) {
        *t = f64::from(z as i32);
        z = (z - *t) * TWO24;
    }
    tx[2] = z;
    let mut nx = 3;
    while tx[nx - 1] == 0.0 {
        /* skip zero term */
        nx -= 1;
    }
    let (n, y0, y1) = kernel_rem_pio2(&tx, e0, nx);
    if hx < 0 { (-n, -y0, -y1) } else { (n, y0, y1) }
}

/// π/2 in 24-bit pieces.
const PIO2: [f64; 8] = [
    1.57079625129699707031e+00,
    7.54978941586159635335e-08,
    5.39030252995776476554e-15,
    3.28200341580791294123e-22,
    1.27065575308067607349e-29,
    1.22933308981111328932e-36,
    2.73370053816464559624e-44,
    2.16741683877804819444e-51,
];

/// Terms of 2/π to keep. fdlibm picks this from the requested precision; `sin` and `cos` always
/// ask for two-piece output (`prec == 2`, `init_jk[2] == 4`), the only case ported here.
const JK: i32 = 4;

/// `__kernel_rem_pio2(x, y, e0, nx, 2, two_over_pi)`: the infinite-precision reduction for
/// arguments past 2^19 (π/2). `x` holds `nx` 24-bit pieces of `|input| * 2^(-e0)`.
fn kernel_rem_pio2(x: &[f64; 3], e0: i32, nx: usize) -> (i32, f64, f64) {
    let jp = JK;
    let jx = nx as i32 - 1;
    let jv = ((e0 - 3) / 24).max(0);
    let mut q0 = e0 - 24 * (jv + 1);

    /* set up f[0] to f[jx+jk] where f[jx+jk] = two_over_pi[jv+jk] */
    let mut f = [0.0f64; 20];
    for (i, fi) in f.iter_mut().take((jx + JK + 1) as usize).enumerate() {
        let j = jv - jx + i as i32;
        *fi = if j < 0 {
            0.0
        } else {
            f64::from(TWO_OVER_PI[j as usize])
        };
    }

    /* compute q[0], q[1], ... q[jk] */
    let mut q = [0.0f64; 20];
    for i in 0..=JK {
        let mut fw = 0.0;
        for j in 0..=jx {
            fw += x[j as usize] * f[(jx + i - j) as usize];
        }
        q[i as usize] = fw;
    }

    let mut iq = [0i32; 20];
    let mut jz = JK;
    let mut z;
    let mut ih;
    let mut n;
    loop {
        /* distill q[] into iq[] reversingly */
        let mut i = 0;
        let mut j = jz;
        z = q[jz as usize];
        while j > 0 {
            let fw = f64::from((TWON24 * z) as i32);
            iq[i as usize] = (z - TWO24 * fw) as i32;
            z = q[(j - 1) as usize] + fw;
            i += 1;
            j -= 1;
        }

        /* compute n */
        z = libm::scalbn(z, q0); /* actual value of z */
        z -= 8.0 * (z * 0.125).floor(); /* trim off integer >= 8 */
        n = z as i32;
        z -= f64::from(n);
        ih = 0;
        if q0 > 0 {
            /* need iq[jz-1] to determine n */
            let i = iq[(jz - 1) as usize] >> (24 - q0);
            n += i;
            iq[(jz - 1) as usize] -= i << (24 - q0);
            ih = iq[(jz - 1) as usize] >> (23 - q0);
        } else if q0 == 0 {
            ih = iq[(jz - 1) as usize] >> 23;
        } else if z >= 0.5 {
            ih = 2;
        }

        if ih > 0 {
            /* q > 0.5 */
            n += 1;
            let mut carry = 0;
            for i in 0..jz {
                /* compute 1 - q */
                let j = iq[i as usize];
                if carry == 0 {
                    if j != 0 {
                        carry = 1;
                        iq[i as usize] = 0x100_0000 - j;
                    }
                } else {
                    iq[i as usize] = 0xff_ffff - j;
                }
            }
            match q0 {
                /* rare case: chance is 1 in 12 */
                1 => iq[(jz - 1) as usize] &= 0x7f_ffff,
                2 => iq[(jz - 1) as usize] &= 0x3f_ffff,
                _ => {}
            }
            if ih == 2 {
                z = 1.0 - z;
                if carry != 0 {
                    z -= libm::scalbn(1.0, q0);
                }
            }
        }

        /* check if recomputation is needed */
        if z != 0.0 {
            break;
        }
        let mut j = 0;
        for i in (JK..jz).rev() {
            j |= iq[i as usize];
        }
        if j != 0 {
            break;
        }
        let mut k = 1;
        while JK >= k && iq[(JK - k) as usize] == 0 {
            /* k = number of terms needed */
            k += 1;
        }
        for i in jz + 1..=jz + k {
            /* add q[jz+1] to q[jz+k] */
            f[(jx + i) as usize] = f64::from(TWO_OVER_PI[(jv + i) as usize]);
            let mut fw = 0.0;
            for j in 0..=jx {
                fw += x[j as usize] * f[(jx + i - j) as usize];
            }
            q[i as usize] = fw;
        }
        jz += k;
    }

    /* chop off zero terms */
    if z == 0.0 {
        jz -= 1;
        q0 -= 24;
        while iq[jz as usize] == 0 {
            jz -= 1;
            q0 -= 24;
        }
    } else {
        /* break z into 24-bit pieces if necessary */
        z = libm::scalbn(z, -q0);
        if z >= TWO24 {
            let fw = f64::from((TWON24 * z) as i32);
            iq[jz as usize] = (z - TWO24 * fw) as i32;
            jz += 1;
            q0 += 24;
            iq[jz as usize] = fw as i32;
        } else {
            iq[jz as usize] = z as i32;
        }
    }

    /* convert integer "bit" chunk to floating-point value */
    let mut fw = libm::scalbn(1.0, q0);
    for i in (0..=jz).rev() {
        q[i as usize] = fw * f64::from(iq[i as usize]);
        fw *= TWON24;
    }

    /* compute PIO2[0,...,jp] * q[jz,...,0] */
    let mut fq = [0.0f64; 20];
    for i in (0..=jz).rev() {
        let mut fw = 0.0;
        let mut k = 0;
        while k <= jp && k <= jz - i {
            fw += PIO2[k as usize] * q[(i + k) as usize];
            k += 1;
        }
        fq[(jz - i) as usize] = fw;
    }

    /* compress fq[] into y[] (prec == 2) */
    let mut fw = 0.0;
    for i in (0..=jz).rev() {
        fw += fq[i as usize];
    }
    let y0 = if ih == 0 { fw } else { -fw };
    let mut tail = fq[0] - fw;
    for i in 1..=jz {
        tail += fq[i as usize];
    }
    let y1 = if ih == 0 { tail } else { -tail };
    (n & 7, y0, y1)
}

// ---------------------------------------------------------------- sin, cos

/// `Math.sin(x)`.
pub fn sin(x: f64) -> f64 {
    let ix = hi(x) & 0x7fff_ffff;
    if ix <= 0x3fe9_21fb {
        /* |x| ~< π/4 */
        return kernel_sin(x, 0.0, 0);
    }
    if ix >= 0x7ff0_0000 {
        /* sin(inf or NaN) is NaN */
        return x - x;
    }
    let (n, y0, y1) = rem_pio2(x);
    match n & 3 {
        0 => kernel_sin(y0, y1, 1),
        1 => kernel_cos(y0, y1),
        2 => -kernel_sin(y0, y1, 1),
        _ => -kernel_cos(y0, y1),
    }
}

/// `Math.cos(x)`.
pub fn cos(x: f64) -> f64 {
    let ix = hi(x) & 0x7fff_ffff;
    if ix <= 0x3fe9_21fb {
        /* |x| ~< π/4 */
        return kernel_cos(x, 0.0);
    }
    if ix >= 0x7ff0_0000 {
        /* cos(inf or NaN) is NaN */
        return x - x;
    }
    let (n, y0, y1) = rem_pio2(x);
    match n & 3 {
        0 => kernel_cos(y0, y1),
        1 => -kernel_sin(y0, y1, 1),
        2 => -kernel_cos(y0, y1),
        _ => kernel_sin(y0, y1, 1),
    }
}

// ---------------------------------------------------------------- pow

const BP: [f64; 2] = [1.0, 1.5];
const DP_H: [f64; 2] = [0.0, 5.84962487220764160156e-01];
const DP_L: [f64; 2] = [0.0, 1.35003920212974897128e-08];
const TWO53: f64 = 9007199254740992.0;
const HUGE: f64 = 1.0e300;
const TINY: f64 = 1.0e-300;
/* poly coefs for (3/2) * (log(x) - 2s - 2/3 s^3) */
const L1: f64 = 5.99999999999994648725e-01;
const L2: f64 = 4.28571428578550184252e-01;
const L3: f64 = 3.33333329818377432918e-01;
const L4: f64 = 2.72728123808534006489e-01;
const L5: f64 = 2.30660745775561754067e-01;
const L6: f64 = 2.06975017800338417784e-01;
const P1: f64 = 1.66666666666666019037e-01;
const P2: f64 = -2.77777777770155933842e-03;
const P3: f64 = 6.61375632143793436117e-05;
const P4: f64 = -1.65339022054652515390e-06;
const P5: f64 = 4.13813679705723846039e-08;
const LG2: f64 = 6.93147180559945286227e-01;
const LG2_H: f64 = 6.93147182464599609375e-01;
const LG2_L: f64 = -1.90465429995776804525e-09;
const OVT: f64 = 8.0085662595372944372e-017;
const CP: f64 = 9.61796693925975554329e-01;
const CP_H: f64 = 9.61796700954437255859e-01;
const CP_L: f64 = -7.02846165095275826516e-09;
const IVLN2: f64 = 1.44269504088896338700e+00;
const IVLN2_H: f64 = 1.44269502162933349609e+00;
const IVLN2_L: f64 = 1.92596299112661746887e-08;

/// `Math.pow(x, y)`. The JavaScript special cases differ from C's: `1 ** NaN` and `(±1) ** ±∞`
/// are NaN, not 1.
pub fn pow(x: f64, y: f64) -> f64 {
    let (hx, lx) = (hi(x), lo(x));
    let (hy, ly) = (hi(y), lo(y));
    let ix = hx & 0x7fff_ffff;
    let iy = hy & 0x7fff_ffff;

    /* y == zero: x**0 = 1 */
    if (iy as u32 | ly) == 0 {
        return 1.0;
    }
    /* ±NaN return x + y */
    if ix > 0x7ff0_0000
        || (ix == 0x7ff0_0000 && lx != 0)
        || iy > 0x7ff0_0000
        || (iy == 0x7ff0_0000 && ly != 0)
    {
        return x + y;
    }

    /* determine if y is an odd int when x < 0
     * yisint = 0 ... y is not an integer
     * yisint = 1 ... y is an odd int
     * yisint = 2 ... y is an even int
     */
    let mut yisint = 0;
    if hx < 0 {
        if iy >= 0x4340_0000 {
            yisint = 2; /* even integer y */
        } else if iy >= 0x3ff0_0000 {
            let k = (iy >> 20) - 0x3ff; /* exponent */
            if k > 20 {
                let j = (ly >> (52 - k)) as i32;
                if j.wrapping_shl((52 - k) as u32) == ly as i32 {
                    yisint = 2 - (j & 1);
                }
            } else if ly == 0 {
                let j = iy >> (20 - k);
                if (j << (20 - k)) == iy {
                    yisint = 2 - (j & 1);
                }
            }
        }
    }

    /* special value of y */
    if ly == 0 {
        if iy == 0x7ff0_0000 {
            /* y is ±inf */
            return if ((ix - 0x3ff0_0000) | lx as i32) == 0 {
                y - y /* (±1)**±inf is NaN */
            } else if ix >= 0x3ff0_0000 {
                /* (|x|>1)**±inf = inf, 0 */
                if hy >= 0 { y } else { 0.0 }
            } else {
                /* (|x|<1)**-,+inf = inf, 0 */
                if hy < 0 { -y } else { 0.0 }
            };
        }
        if iy == 0x3ff0_0000 {
            /* y is ±1 */
            return if hy < 0 { 1.0 / x } else { x };
        }
        if hy == 0x4000_0000 {
            /* y is 2 */
            return x * x;
        }
        if hy == 0x3fe0_0000 && hx >= 0 {
            /* y is 0.5, x >= +0 */
            return x.sqrt();
        }
    }

    let mut ax = x.abs();
    /* special value of x */
    if lx == 0 && (ix == 0x7ff0_0000 || ix == 0 || ix == 0x3ff0_0000) {
        /* x is ±0, ±inf, ±1 */
        let mut z = ax;
        if hy < 0 {
            z = 1.0 / z; /* z = 1/|x| */
        }
        if hx < 0 {
            if ((ix - 0x3ff0_0000) | yisint) == 0 {
                z = f64::NAN; /* (-1)**non-int is NaN */
            } else if yisint == 1 {
                z = -z; /* (x<0)**odd = -(|x|**odd) */
            }
        }
        return z;
    }

    let mut n = (hx >> 31) + 1;
    /* (x<0)**(non-int) is NaN */
    if (n | yisint) == 0 {
        return f64::NAN;
    }
    let mut s = 1.0; /* sign of result: -1 for (-ve)**odd, else 1 */
    if (n | (yisint - 1)) == 0 {
        s = -1.0;
    }

    let (t1, t2);
    if iy > 0x41e0_0000 {
        /* |y| > 2**31 */
        if iy > 0x43f0_0000 {
            /* |y| > 2**64, must over/underflow */
            if ix <= 0x3fef_ffff {
                return if hy < 0 { HUGE * HUGE } else { TINY * TINY };
            }
            if ix >= 0x3ff0_0000 {
                return if hy > 0 { HUGE * HUGE } else { TINY * TINY };
            }
        }
        /* over/underflow if x is not close to one */
        if ix < 0x3fef_ffff {
            return if hy < 0 {
                s * HUGE * HUGE
            } else {
                s * TINY * TINY
            };
        }
        if ix > 0x3ff0_0000 {
            return if hy > 0 {
                s * HUGE * HUGE
            } else {
                s * TINY * TINY
            };
        }
        /* now |1-x| is tiny <= 2**-20; log(x) as x - x^2/2 + x^3/3 - x^4/4 suffices */
        let t = ax - 1.0; /* t has 20 trailing zeros */
        let w = (t * t) * (0.5 - t * (0.3333333333333333333333 - t * 0.25));
        let u = IVLN2_H * t; /* IVLN2_H has 21 significant bits */
        let v = t * IVLN2_L - w * IVLN2;
        t1 = with_lo(u + v, 0);
        t2 = v - (t1 - u);
    } else {
        n = 0;
        let mut ix = ix;
        if ix < 0x0010_0000 {
            /* take care of subnormal number */
            ax *= TWO53;
            n -= 53;
            ix = hi(ax);
        }
        n += (ix >> 20) - 0x3ff;
        let j = ix & 0x000f_ffff;
        /* determine interval */
        ix = j | 0x3ff0_0000; /* normalize ix */
        let k = if j <= 0x3988E {
            0 /* |x| < sqrt(3/2) */
        } else if j < 0xBB67A {
            1 /* |x| < sqrt(3) */
        } else {
            n += 1;
            ix -= 0x0010_0000;
            0
        };
        ax = with_hi(ax, ix);

        /* compute ss = s_h + s_l = (x-1)/(x+1) or (x-1.5)/(x+1.5) */
        let u = ax - BP[k]; /* BP[0] = 1.0, BP[1] = 1.5 */
        let v = 1.0 / (ax + BP[k]);
        let ss = u * v;
        let s_h = with_lo(ss, 0);
        /* t_h = ax + BP[k] high */
        let t_h = words(
            ((ix >> 1) | 0x2000_0000) + 0x0008_0000 + ((k as i32) << 18),
            0,
        );
        let t_l = ax - (t_h - BP[k]);
        let s_l = v * ((u - s_h * t_h) - s_h * t_l);
        /* compute log(ax) */
        let s2 = ss * ss;
        let mut r = s2 * s2 * (L1 + s2 * (L2 + s2 * (L3 + s2 * (L4 + s2 * (L5 + s2 * L6)))));
        r += s_l * (s_h + ss);
        let s2 = s_h * s_h;
        let t_h = with_lo(3.0 + s2 + r, 0);
        let t_l = r - ((t_h - 3.0) - s2);
        /* u + v = ss * (1 + ...) */
        let u = s_h * t_h;
        let v = s_l * t_h + t_l * ss;
        /* 2/(3 log2) * (ss + ...) */
        let p_h = with_lo(u + v, 0);
        let p_l = v - (p_h - u);
        let z_h = CP_H * p_h; /* CP_H + CP_L = 2/(3 log2) */
        let z_l = CP_L * p_h + p_l * CP + DP_L[k];
        /* log2(ax) = (ss + ..) * 2/(3 log2) = n + DP_H + z_h + z_l */
        let t = f64::from(n);
        t1 = with_lo(((z_h + z_l) + DP_H[k]) + t, 0);
        t2 = z_l - (((t1 - t) - DP_H[k]) - z_h);
    }

    /* split up y into y1 + y2 and compute (y1 + y2) * (t1 + t2) */
    let y1 = with_lo(y, 0);
    let p_l = (y - y1) * t1 + y * t2;
    let mut p_h = y1 * t1;
    let z = p_l + p_h;
    let (mut j, i) = (hi(z), lo(z) as i32);
    if j >= 0x4090_0000 {
        /* z >= 1024 */
        if ((j - 0x4090_0000) | i) != 0 {
            return s * HUGE * HUGE; /* overflow */
        }
        if p_l + OVT > z - p_h {
            return s * HUGE * HUGE; /* overflow */
        }
    } else if (j & 0x7fff_ffff) >= 0x4090_cc00 {
        /* z <= -1075 */
        if ((j as u32).wrapping_sub(0xc090_cc00) | i as u32) != 0 {
            return s * TINY * TINY; /* underflow */
        }
        if p_l <= z - p_h {
            return s * TINY * TINY; /* underflow */
        }
    }

    /* compute 2**(p_h + p_l) */
    let i = j & 0x7fff_ffff;
    let mut k = (i >> 20) - 0x3ff;
    n = 0;
    if i > 0x3fe0_0000 {
        /* if |z| > 0.5, set n = [z+0.5] */
        n = j + (0x0010_0000 >> (k + 1));
        k = ((n & 0x7fff_ffff) >> 20) - 0x3ff; /* new k for n */
        let t = words(n & !(0x000f_ffff >> k), 0);
        n = ((n & 0x000f_ffff) | 0x0010_0000) >> (20 - k);
        if j < 0 {
            n = -n;
        }
        p_h -= t;
    }
    let t = with_lo(p_l + p_h, 0);
    let u = t * LG2_H;
    let v = (p_l - (t - p_h)) * LG2 + t * LG2_L;
    let mut z = u + v;
    let w = v - (z - u);
    let t = z * z;
    let t1 = z - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
    /* V8 brackets this as one division; fdlibm divides only by (t1 - 2) and subtracts after.
     * The difference is the last bit of about 4 % of results, so it has to be reproduced. */
    let r = (z * t1) / ((t1 - 2.0) - (w + z * w));
    z = 1.0 - (r - z);
    j = hi(z);
    j = j.wrapping_add(((n as u32) << 20) as i32);
    if (j >> 20) <= 0 {
        z = libm::scalbn(z, n); /* subnormal output */
    } else {
        z = with_hi(z, j);
    }
    s * z
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    /// Every expected value below is what V8 prints for the same argument.
    #[test]
    fn sin_and_cos_cover_all_three_reduction_paths() {
        assert_eq!(sin(0.5), 0.479425538604203); /* |x| <= pi/4: straight to the kernel */
        assert_eq!(sin(2.0), 0.9092974268256817); /* |x| < 3pi/4: one subtraction of pi/2 */
        assert_eq!(cos(2.0), -0.4161468365471424);
        assert_eq!(sin(43758.5453), 0.643277862396871); /* medium: n * pi/2 in three steps */
        assert_eq!(cos(43758.5453), -0.765632804776619);
        assert_eq!(sin(823549.6621622349), -0.002420405505153685); /* 2^19 * (pi/2) */
        assert_eq!(sin(1e22), -0.8522008497671888); /* past 2^19 * (pi/2): the 2/pi table */
        assert_eq!(cos(1e22), 0.523214785395139);
        assert_eq!(sin(5.319372648326541e255), 1.0); /* the worst case for the reduction */
        assert_eq!(cos(5.319372648326541e255), -4.687165924254628e-19);
    }

    #[test]
    fn sin_and_cos_keep_javascript_signs_and_specials() {
        assert_eq!(sin(0.0), 0.0);
        assert!(sin(-0.0).is_sign_negative());
        assert_eq!(cos(-0.0), 1.0);
        assert_eq!(sin(5e-324), 5e-324); /* smallest subnormal, returned unchanged */
        assert!(sin(f64::INFINITY).is_nan());
        assert!(cos(f64::NEG_INFINITY).is_nan());
        assert!(sin(f64::NAN).is_nan());
    }

    #[test]
    fn pow_follows_the_ecmascript_special_cases() {
        assert_eq!(pow(f64::NAN, 0.0), 1.0); /* anything ** 0 is 1, NaN included */
        assert!(pow(1.0, f64::NAN).is_nan()); /* 1 ** NaN is NaN, unlike C's pow */
        assert!(pow(1.0, f64::INFINITY).is_nan());
        assert!(pow(-1.0, f64::NEG_INFINITY).is_nan());
        assert_eq!(pow(-2.0, 3.0), -8.0);
        assert!(pow(-2.0, 2.5).is_nan()); /* (-ve) ** non-integer */
        assert_eq!(pow(0.0, -1.0), f64::INFINITY);
        assert_eq!(pow(-0.0, -3.0), f64::NEG_INFINITY);
        assert_eq!(pow(2.0, 0.5), std::f64::consts::SQRT_2);
        assert_eq!(pow(2.0, 1024.0), f64::INFINITY); /* overflow */
        assert_eq!(pow(2.0, -1074.0), 5e-324); /* subnormal output */
        assert_eq!(pow(2.0, -1075.0), 0.0); /* underflow */
        assert_eq!(pow(1.0000000000000002, 1e17), 4398196873.9457445); /* |y| > 2^31, x near 1 */
    }

    /// The last bit V8 and `libm` disagree on, and the reason this module exists: V8 divides by
    /// `(t1 - 2) - (w + z * w)` where fdlibm divides by `t1 - 2` and subtracts afterwards.
    #[test]
    #[allow(clippy::disallowed_methods)] // the point of the test is what `libm` returns
    fn pow_keeps_v8s_bracketing() {
        assert_eq!(pow(0.9405010254122317, 2.2), 0.8737564636136465);
        assert_eq!(
            pow(0.9405010254122317, 2.2).to_bits(),
            0x3feb_f5d0_1d7c_7489
        );
        assert_eq!(
            libm::pow(0.9405010254122317, 2.2).to_bits(),
            0x3feb_f5d0_1d7c_748a,
            "libm rounds this the other way; if it ever stops, the note in README.md is stale"
        );
    }
}
