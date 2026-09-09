use crate::TileId;

/// Adjacency of a playing surface, all that `cl-rules` needs from geometry. The hex sphere
/// implements it; tests use hand-made graphs.
pub trait Board {
    /// Number of tiles; ids are `0..tile_count()`.
    fn tile_count(&self) -> usize;

    /// Tiles sharing an edge with `id`, in a stable order.
    fn neighbors(&self, id: TileId) -> &[TileId];

    /// Every id in order.
    fn ids(&self) -> Box<dyn Iterator<Item = TileId> + '_> {
        Box::new((0..self.tile_count()).map(|i| TileId(i as u32)))
    }
}

/// A board given by explicit neighbour lists; the test double for rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphBoard {
    neighbors: Vec<Vec<TileId>>,
}

impl GraphBoard {
    /// Builds from undirected edges; each edge is inserted in both directions in the order given.
    pub fn from_edges(tile_count: usize, edges: &[(u32, u32)]) -> Self {
        let mut neighbors = vec![Vec::new(); tile_count];
        for &(a, b) in edges {
            neighbors[a as usize].push(TileId(b));
            neighbors[b as usize].push(TileId(a));
        }
        Self { neighbors }
    }

    /// A centre tile `0` ringed by six tiles `1..=6`, each ring tile joined to its two ring
    /// neighbours: the smallest board on which every rule can be exercised.
    pub fn flower() -> Self {
        let mut edges = Vec::new();
        for i in 1..=6u32 {
            edges.push((0, i));
            edges.push((i, if i == 6 { 1 } else { i + 1 }));
        }
        Self::from_edges(7, &edges)
    }
}

impl Board for GraphBoard {
    fn tile_count(&self) -> usize {
        self.neighbors.len()
    }

    fn neighbors(&self, id: TileId) -> &[TileId] {
        &self.neighbors[id.index()]
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn flower_has_hex_centre_and_ring() {
        let b = GraphBoard::flower();
        assert_eq!(b.tile_count(), 7);
        assert_eq!(b.neighbors(TileId(0)).len(), 6);
        for i in 1..=6 {
            assert_eq!(b.neighbors(TileId(i)).len(), 3, "ring tile {i}");
        }
        assert_eq!(b.ids().count(), 7);
    }
}
