//! The one clock in the game.
//!
//! `docs/design/porting.md` keeps time out of every other crate: generation must not depend on it,
//! and only the animation does. On the web that clock is `performance.now()`, because
//! `std::time::Instant` panics on `wasm32-unknown-unknown`.

/// Milliseconds since some fixed origin. Only differences are meaningful.
#[cfg(not(target_arch = "wasm32"))]
pub fn now_ms() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

/// Milliseconds since the page's time origin.
#[cfg(target_arch = "wasm32")]
pub fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or(0.0, |p| p.now())
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_starts_at_zero_and_never_goes_back() {
        let a = now_ms();
        let b = now_ms();
        assert!(a >= 0.0);
        assert!(b >= a, "monotonic");
    }
}
