use ndarray::Array5;
use serde::{Deserialize, Serialize};

use crate::inference::osm_buildings::OsmBuildingWay;
use crate::inference::overlay_geo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometryBlock {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[serde(alias = "surfaceType")]
    pub surface_type: String,
}

impl GeometryBlock {
    #[allow(dead_code)]
    pub fn position(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

const SURFACE_TYPES: [&str; 6] = ["bitume", "herbe", "eau", "gravier", "vegetation", "batiment"];

/// Build a 15-channel 3-D voxel tensor from weather, optional OSM footprints, then user blocks.
///
/// `geometry[].x/y` = **lon / lat** WGS84 ; `z` = altitude (m). Aligné avec `overlay_geo` et le front.
///
/// Output shape: `[1, 15, NX, NY, NZ]`
///
/// Channel layout (MUST match training/src/model/encoding.py):
///   0     : occupancy (binary — 1 = solid, 0 = air)
///   1–6   : surface type one-hot (bitume, herbe, eau, gravier, vegetation, batiment)
///   7     : albedo α
///   8     : emissivity ε
///   9     : roughness z₀
///   10    : wind speed (broadcast)
///   11    : sin(wind_dir) (broadcast)
///   12    : cos(wind_dir) (broadcast)
///   13    : sun elevation (broadcast)
///   14    : ambient temperature (broadcast)
pub fn preprocess_geometry(
    geometry: &[GeometryBlock],
    wind_speed: f64,
    wind_dir: f64,
    sun_elevation: f64,
    t_ambient: f64,
    osm_ways: Option<&[OsmBuildingWay]>,
) -> Array5<f32> {
    let mut tensor = build_base_tensor(wind_speed, wind_dir, sun_elevation, t_ambient);
    if let Some(ways) = osm_ways {
        crate::inference::osm_buildings::rasterize_ways_into_tensor(&mut tensor, ways);
    }
    apply_geometry_blocks(&mut tensor, geometry);
    tensor
}

pub fn build_base_tensor(
    wind_speed: f64,
    wind_dir: f64,
    sun_elevation: f64,
    t_ambient: f64,
) -> Array5<f32> {
    let (nx, ny, nz) = (overlay_geo::GRID_NX, overlay_geo::GRID_NY, overlay_geo::GRID_NZ);
    let nch = 15;
    let mut tensor = Array5::<f32>::zeros((1, nch, nx, ny, nz));

    let wind_dir_rad = wind_dir.to_radians();
    let sin_dir = wind_dir_rad.sin() as f32;
    let cos_dir = wind_dir_rad.cos() as f32;
    let ws = wind_speed as f32;
    let sun = sun_elevation as f32;
    let ta = t_ambient as f32;

    tensor.slice_mut(ndarray::s![0, 10, .., .., ..]).fill(ws);
    tensor.slice_mut(ndarray::s![0, 11, .., .., ..]).fill(sin_dir);
    tensor.slice_mut(ndarray::s![0, 12, .., .., ..]).fill(cos_dir);
    tensor.slice_mut(ndarray::s![0, 13, .., .., ..]).fill(sun);
    tensor.slice_mut(ndarray::s![0, 14, .., .., ..]).fill(ta);

    tensor
}

pub fn apply_geometry_blocks(tensor: &mut Array5<f32>, geometry: &[GeometryBlock]) {
    for block in geometry {
        let (ix, iy) = overlay_geo::grid_ix_iy(block.x, block.y);
        let iz = overlay_geo::grid_iz(block.z);

        tensor[[0, 0, ix, iy, iz]] = 1.0;

        let type_idx = surface_type_index(&block.surface_type);
        for c in 1..=6 {
            tensor[[0, c, ix, iy, iz]] = 0.0;
        }
        tensor[[0, 1 + type_idx, ix, iy, iz]] = 1.0;

        let (albedo, emissivity, roughness) = surface_physical_props(&block.surface_type);
        tensor[[0, 7, ix, iy, iz]] = albedo;
        tensor[[0, 8, ix, iy, iz]] = emissivity;
        tensor[[0, 9, ix, iy, iz]] = roughness;
    }
}

fn surface_type_index(surface_type: &str) -> usize {
    let normalized = match surface_type {
        "asphalt" => "bitume",
        "grass" | "vegetation" => "herbe",
        "water" => "eau",
        "gravel" => "gravier",
        "building" | "concrete" | "glass" | "metal" | "brick" => "batiment",
        other => other,
    };
    SURFACE_TYPES
        .iter()
        .position(|&s| s == normalized)
        .unwrap_or(5) // default to batiment
}

/// (albedo, emissivity, roughness_z0)
fn surface_physical_props(surface_type: &str) -> (f32, f32, f32) {
    match surface_type {
        "bitume" | "asphalt" => (0.1, 0.95, 0.01),
        "herbe" | "grass" | "vegetation" => (0.25, 0.95, 0.1),
        "eau" | "water" => (0.06, 0.96, 0.0001),
        "gravier" | "gravel" => (0.2, 0.90, 0.015),
        "batiment" | "building" | "concrete" => (0.3, 0.90, 0.02),
        "glass" => (0.4, 0.84, 0.001),
        "metal" => (0.6, 0.20, 0.005),
        "brick" => (0.3, 0.90, 0.03),
        _ => (0.3, 0.90, 0.02),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_geometry_fills_broadcast_channels() {
        let t = preprocess_geometry(&[], 5.0, 90.0, 30.0, 18.0, None);
        assert_eq!(t.shape(), &[1, 15, 256, 256, 64]);
        let wind_rad = 90f64.to_radians();
        assert!((t[[0, 10, 10, 10, 10]] - 5.0).abs() < 1e-6);
        assert!((t[[0, 11, 0, 0, 0]] - wind_rad.sin() as f32).abs() < 1e-5);
        assert!((t[[0, 12, 0, 0, 0]] - wind_rad.cos() as f32).abs() < 1e-5);
        assert!((t[[0, 13, 0, 0, 0]] - 30.0).abs() < 1e-6);
        assert!((t[[0, 14, 0, 0, 0]] - 18.0).abs() < 1e-6);
    }

    #[test]
    fn block_sets_occupancy_and_surface_one_hot_at_paris_origin_cell() {
        let g = [GeometryBlock {
            x: overlay_geo::OVERLAY_ORIGIN_LON,
            y: overlay_geo::OVERLAY_ORIGIN_LAT,
            z: 0.0,
            surface_type: "bitume".into(),
        }];
        let t = preprocess_geometry(&g, 1.0, 0.0, 0.0, 20.0, None);
        assert_eq!(t[[0, 0, 0, 0, 0]], 1.0);
        assert_eq!(t[[0, 1, 0, 0, 0]], 1.0);
        for c in 2..=6 {
            assert_eq!(t[[0, c, 0, 0, 0]], 0.0, "ch {}", c);
        }
        assert!((t[[0, 7, 0, 0, 0]] - 0.1).abs() < 1e-6);
    }

    #[test]
    fn surface_type_index_maps_aliases() {
        assert_eq!(surface_type_index("asphalt"), surface_type_index("bitume"));
        assert_eq!(surface_type_index("grass"), surface_type_index("herbe"));
    }
}
