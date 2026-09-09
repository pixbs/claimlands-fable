//! Icosahedron and its frequency-`n` geodesic subdivision, prototype section 1.

use std::collections::BTreeMap;

use cl_noise::js::to_fixed6;
use cl_noise::vec::{V3, norm};

/// Unit icosahedron: 12 vertices and 20 CCW faces in the prototype's order.
pub fn icosahedron() -> (Vec<V3>, Vec<[usize; 3]>) {
    let t = (1.0 + 5.0f64.sqrt()) / 2.0;
    let v = [
        [-1.0, t, 0.0],
        [1.0, t, 0.0],
        [-1.0, -t, 0.0],
        [1.0, -t, 0.0],
        [0.0, -1.0, t],
        [0.0, 1.0, t],
        [0.0, -1.0, -t],
        [0.0, 1.0, -t],
        [t, 0.0, -1.0],
        [t, 0.0, 1.0],
        [-t, 0.0, -1.0],
        [-t, 0.0, 1.0],
    ]
    .into_iter()
    .map(norm)
    .collect();
    let f = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    (v, f)
}

/// A subdivided icosahedron projected onto the unit sphere.
#[derive(Debug, Clone, PartialEq)]
pub struct Geodesic {
    /// Unit vertices, deduplicated across faces; index order is the tile id order.
    pub verts: Vec<V3>,
    /// Triangles as vertex indices.
    pub tris: Vec<[u32; 3]>,
}

/// The prototype's vertex key: components under `1e-9` snap to zero, then `toFixed(6)`.
fn key(p: V3) -> String {
    let part = |x: f64| to_fixed6(if x.abs() < 1e-9 { 0.0 } else { x });
    format!("{},{},{}", part(p[0]), part(p[1]), part(p[2]))
}

/// Subdivides every icosahedron face into `n²` triangles and projects the vertices onto the sphere.
/// Shared vertices are merged by `key`, first occurrence wins, exactly as the prototype does, so
/// vertex (and therefore tile) ids match it.
pub fn geodesic(n: u8) -> Geodesic {
    let (base_v, base_f) = icosahedron();
    let n_f = f64::from(n);
    let nu = usize::from(n);
    let mut verts: Vec<V3> = Vec::new();
    let mut index: BTreeMap<String, u32> = BTreeMap::new();
    let mut tris: Vec<[u32; 3]> = Vec::new();
    let mut push = |p: V3| -> u32 {
        let k = key(p);
        if let Some(&i) = index.get(&k) {
            return i;
        }
        let i = verts.len() as u32;
        index.insert(k, i);
        verts.push(p);
        i
    };
    for [ia, ib, ic] in base_f {
        let (a, b, c) = (base_v[ia], base_v[ib], base_v[ic]);
        let mut g: Vec<Vec<u32>> = Vec::with_capacity(nu + 1);
        for i in 0..=nu {
            let mut row = Vec::with_capacity(nu + 1 - i);
            for j in 0..=(nu - i) {
                let w = (nu - i - j) as f64;
                let (fi, fj) = (i as f64, j as f64);
                row.push(push(norm([
                    (a[0] * w + b[0] * fi + c[0] * fj) / n_f,
                    (a[1] * w + b[1] * fi + c[1] * fj) / n_f,
                    (a[2] * w + b[2] * fi + c[2] * fj) / n_f,
                ])));
            }
            g.push(row);
        }
        for i in 0..nu {
            for j in 0..(nu - i) {
                tris.push([g[i][j], g[i + 1][j], g[i][j + 1]]);
                if j < nu - i - 1 {
                    tris.push([g[i + 1][j], g[i + 1][j + 1], g[i][j + 1]]);
                }
            }
        }
    }
    Geodesic { verts, tris }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;

    #[test]
    fn counts_follow_euler() {
        for n in 2..=6u8 {
            let g = geodesic(n);
            let n2 = usize::from(n) * usize::from(n);
            assert_eq!(g.verts.len(), 10 * n2 + 2, "n={n}");
            assert_eq!(g.tris.len(), 20 * n2, "n={n}");
        }
    }
}
