use crate::TileId;

/// A non-indexed triangle list with per-vertex attributes, exactly what the prototype pushes into
/// its `BufferGeometry`s. Non-indexed keeps every tile's UVs and colours independent and makes
/// triangle index → tile id a flat lookup.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshData {
    /// `xyz` per vertex.
    pub positions: Vec<f32>,
    /// `xyz` per vertex, one flat normal per triangle repeated three times.
    pub normals: Vec<f32>,
    /// `uv` per vertex, or empty for untextured meshes.
    pub uvs: Vec<f32>,
    /// `rgb` per vertex, or empty when the material colour alone applies.
    pub colors: Vec<f32>,
    /// Tile owning each triangle, or empty when picking does not apply.
    pub face_tile: Vec<TileId>,
}

impl MeshData {
    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.positions.len() / 9
    }

    /// `true` when there is nothing to draw.
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Appends one vertex with position and normal; uvs and colours are pushed by the caller when
    /// the mesh carries them.
    pub fn push_vertex(&mut self, p: [f64; 3], n: [f64; 3]) {
        self.positions
            .extend([p[0] as f32, p[1] as f32, p[2] as f32]);
        self.normals.extend([n[0] as f32, n[1] as f32, n[2] as f32]);
    }

    /// Attribute lengths agree with each other; `Err` names the first mismatch.
    pub fn validate(&self) -> Result<(), String> {
        let v = self.positions.len();
        if !v.is_multiple_of(9) {
            return Err(format!("positions length {v} is not a multiple of 9"));
        }
        if self.normals.len() != v {
            return Err(format!(
                "normals length {} != positions length {v}",
                self.normals.len()
            ));
        }
        if !self.uvs.is_empty() && self.uvs.len() != v / 3 * 2 {
            return Err(format!(
                "uvs length {} for {} vertices",
                self.uvs.len(),
                v / 3
            ));
        }
        if !self.colors.is_empty() && self.colors.len() != v {
            return Err(format!(
                "colors length {} != positions length {v}",
                self.colors.len()
            ));
        }
        if !self.face_tile.is_empty() && self.face_tile.len() != v / 9 {
            return Err(format!(
                "face_tile length {} for {} triangles",
                self.face_tile.len(),
                v / 9
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn validate_checks_every_attribute() {
        let mut m = MeshData::default();
        assert!(m.is_empty() && m.validate().is_ok());
        for _ in 0..3 {
            m.push_vertex([0.0, 1.0, 2.0], [0.0, 0.0, 1.0]);
        }
        assert_eq!((m.vertex_count(), m.triangle_count()), (3, 1));
        assert!(m.validate().is_ok());
        m.uvs = vec![0.0; 5];
        assert!(m.validate().unwrap_err().starts_with("uvs"));
        m.uvs.clear();
        m.face_tile = vec![TileId(0), TileId(1)];
        assert!(m.validate().unwrap_err().starts_with("face_tile"));
    }
}
