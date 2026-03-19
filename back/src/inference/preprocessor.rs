use ndarray::Array5;
use serde::{Deserialize, Serialize};

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

/// Build a 15-channel 3-D voxel tensor from geometry blocks and weather conditions.
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
) -> Array5<f32> {
    let (nx, ny, nz) = (256, 256, 64);
    let nch = 15;
    let mut tensor = Array5::<f32>::zeros((1, nch, nx, ny, nz));

    let wind_dir_rad = wind_dir.to_radians();
    let sin_dir = wind_dir_rad.sin() as f32;
    let cos_dir = wind_dir_rad.cos() as f32;
    let ws = wind_speed as f32;
    let sun = sun_elevation as f32;
    let ta = t_ambient as f32;

    {
        let mut ch10 = tensor.slice_mut(ndarray::s![0, 10, .., .., ..]);
        ch10.fill(ws);
    }
    {
        let mut ch11 = tensor.slice_mut(ndarray::s![0, 11, .., .., ..]);
        ch11.fill(sin_dir);
    }
    {
        let mut ch12 = tensor.slice_mut(ndarray::s![0, 12, .., .., ..]);
        ch12.fill(cos_dir);
    }
    {
        let mut ch13 = tensor.slice_mut(ndarray::s![0, 13, .., .., ..]);
        ch13.fill(sun);
    }
    {
        let mut ch14 = tensor.slice_mut(ndarray::s![0, 14, .., .., ..]);
        ch14.fill(ta);
    }

    for block in geometry {
        let gx = (block.x as i64).clamp(0, (nx as i64) - 1) as usize;
        let gy = (block.y as i64).clamp(0, (ny as i64) - 1) as usize;
        let gz = (block.z as i64).clamp(0, (nz as i64) - 1) as usize;

        tensor[[0, 0, gx, gy, gz]] = 1.0;

        let type_idx = surface_type_index(&block.surface_type);
        tensor[[0, 1 + type_idx, gx, gy, gz]] = 1.0;

        let (albedo, emissivity, roughness) = surface_physical_props(&block.surface_type);
        tensor[[0, 7, gx, gy, gz]] = albedo;
        tensor[[0, 8, gx, gy, gz]] = emissivity;
        tensor[[0, 9, gx, gy, gz]] = roughness;
    }

    tensor
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
