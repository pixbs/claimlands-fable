//! Scale constants shared by geometry, textures and rendering. Values are the prototype's; a change
//! here changes every fixture, so it is always a deliberate `visual-change`.

/// Smallest hex sphere that still has hexagons (frequency 1 is a dodecahedron).
pub const MIN_FREQUENCY: u8 = 2;
/// Largest supported frequency: 1442 tiles.
pub const MAX_FREQUENCY: u8 = 12;
/// Radius of the land shell in world units.
pub const RADIUS: f64 = 1.0;
/// Side of one tile's cell in the ground atlas, in texture pixels.
pub const TILE_PX: usize = 24;
/// Fraction of the cell the UVs reach, keeping nearest sampling off the atlas seam.
pub const UV_INSET: f64 = 0.94;
/// Cliff height per elevation level, in texture pixels.
pub const LEVEL_PX: f64 = 4.0;
/// Atmosphere shell height above the land shell, in texture pixels.
pub const ATMO_PX: f64 = 12.0;
/// Lowest cloud deck above the atmosphere, in texture pixels.
pub const CLOUD_PX: f64 = 8.0;

/// Number of tiles on a hex sphere of frequency `n`: always `10n² + 2`, twelve of them pentagons.
pub fn tile_count(n: u8) -> usize {
    let n = usize::from(n);
    10 * n * n + 2
}

/// `true` when `n` is a supported frequency.
pub fn frequency_is_valid(n: u8) -> bool {
    (MIN_FREQUENCY..=MAX_FREQUENCY).contains(&n)
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn tile_counts_match_the_prototype_slider() {
        assert_eq!(tile_count(2), 42);
        assert_eq!(tile_count(8), 642);
        assert_eq!(tile_count(12), 1442);
        assert!(frequency_is_valid(2) && frequency_is_valid(12));
        assert!(!frequency_is_valid(1) && !frequency_is_valid(13));
    }
}
