//! The prototype's `mulberry32`, the only generator behind continents, cover seeding and parcels.

use crate::js::{to_int32, ushr};

/// A 32-bit generator with the prototype's exact sequence for every seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mulberry32 {
    state: i32,
}

impl Mulberry32 {
    /// `mulberry32(seed)`: the seed goes through `ToInt32`, like `a |= 0`.
    pub fn new(seed: f64) -> Self {
        Self {
            state: to_int32(seed),
        }
    }

    /// Seed from an already-wrapped 32-bit value.
    pub fn from_i32(seed: i32) -> Self {
        Self { state: seed }
    }

    /// Next value in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        let a = self.state.wrapping_add(0x6D2B_79F5);
        self.state = a;
        let mut t = (a ^ (ushr(a, 15) as i32)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ (ushr(t, 7) as i32)).wrapping_mul(0x3d | t)) ^ t; // 61 | t in the prototype
        f64::from(ushr(t ^ (ushr(t, 14) as i32), 0)) / 4_294_967_296.0
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn sequence_is_deterministic_and_bounded() {
        let mut a = Mulberry32::new(31676.0);
        let mut b = Mulberry32::new(31676.0);
        for _ in 0..100 {
            let x = a.next_f64();
            assert_eq!(x, b.next_f64());
            assert!((0.0..1.0).contains(&x));
        }
    }
}
