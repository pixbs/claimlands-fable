//! Colour and band constants, verbatim from the prototype. Each comment states why the value is
//! what it is; a change here is a `visual-change`.

/// Cliff rows, top first: bare soil under the grass, down into wet sand. A green top row read as a
/// second stripe of grass.
pub const CLIFF_ROWS: [&str; 4] = ["#5a4a2e", "#7d6a45", "#a08a5f", "#bda677"];
/// Surf rows, shore side first.
pub const FOAM_ROWS: [&str; 4] = ["#ffffff", "#eaf6ff", "#d3e8f7", "#bcd9ee"];
/// Ground bands, dark to light: shadow, mid, highlight. Quantising a smooth field into these with
/// a dithered edge is what gives the patchwork look.
pub const GRASS_BANDS: [&str; 3] = ["#3f7d34", "#5aa444", "#7cc255"];
/// Ground under a village: grass walked off into bare earth, with the asked-for tone in the middle.
pub const MUD_BANDS: [&str; 3] = ["#a18a5b", "#c9ac72", "#ddbd7d"];
/// Sea bands, deep to shallow.
pub const SEA_BANDS: [&str; 5] = ["#123659", "#1a4d78", "#246698", "#3184b0", "#46a2c2"];
/// One flat tone for the atmosphere rim and the halo, so they read as one body of air.
pub const AIR_COLOR: &str = "#c3e3f6";

/// One crop of farmland: flat colour, a darker furrow line, the sloped side's colour, the height of
/// the top in world pixels, and its share of parcels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crop {
    /// Name, for debugging.
    pub key: &'static str,
    /// Flat top colour.
    pub base: &'static str,
    /// Furrow line colour.
    pub furrow: &'static str,
    /// Sloped side colour.
    pub edge: &'static str,
    /// Height of the top surface in world pixels; the side runs out the same distance.
    pub h: f64,
    /// Weight when picking a crop for a parcel.
    pub w: f64,
}

/// Straight from the reference art: a field is one flat colour with a thin darker line every few
/// pixels.
pub const FIELD_CROPS: [Crop; 4] = [
    Crop {
        key: "dirt",
        base: "#a57b4b",
        furrow: "#84623f",
        edge: "#8f6a40",
        h: 1.0,
        w: 0.26,
    },
    Crop {
        key: "green",
        base: "#51982e",
        furrow: "#427d24",
        edge: "#468228",
        h: 1.5,
        w: 0.22,
    },
    Crop {
        key: "olive",
        base: "#c0c729",
        furrow: "#a5c035",
        edge: "#a9af24",
        h: 1.5,
        w: 0.22,
    },
    Crop {
        key: "yellow",
        base: "#f1d534",
        furrow: "#dcbe20",
        edge: "#d7bd2c",
        h: 1.5,
        w: 0.30,
    },
];
