//! The prototype's `V` kit on `[f64; 3]`. Lengths use V8's `Math.hypot`, so normalised vectors
//! match the prototype to the last bit.

use crate::js::hypot3;

/// A 3-vector in double precision; generation math stays in `f64` until GPU upload.
pub type V3 = [f64; 3];

/// `a - b`.
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// `a + b`.
pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// `a * s`.
pub fn mul(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// `a · b`, summed left to right.
pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// `a × b`.
pub fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// `Math.hypot(x, y, z)`.
pub fn len(a: V3) -> f64 {
    hypot3(a[0], a[1], a[2])
}

/// Unit vector; a zero (or NaN) length divides by 1 instead, like `this.len(a) || 1`.
pub fn norm(a: V3) -> V3 {
    let l = len(a);
    let l = if l == 0.0 || l.is_nan() { 1.0 } else { l };
    [a[0] / l, a[1] / l, a[2] / l]
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        assert_eq!(cross([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
        assert_eq!(norm([0.0, 0.0, 0.0]), [0.0, 0.0, 0.0]);
        assert_eq!(len([3.0, 4.0, 0.0]), 5.0);
        assert_eq!(
            dot(
                add([1.0, 2.0, 3.0], [1.0, 1.0, 1.0]),
                mul([1.0, 1.0, 1.0], 2.0)
            ),
            18.0
        );
        assert_eq!(sub([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]), [0.0, 0.0, 0.0]);
    }
}
