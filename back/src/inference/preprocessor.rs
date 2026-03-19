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

/// Build a 15-channel 3-D voxel tensor from geometry blocks and weather conditions.
///
/// Output shape: `[1, 15, NX, NY, NZ]`
///
/// Channel layout:
///   0  building/solid mask
///   1  surface albedo
///   2  surface emissivity
///   3  thermal conductivity
///   4  roughness length
///   5  vegetation mask
///   6  initial wind u
///   7  initial wind v
///   8  initial wind w (vertical, zero-init)
///   9  initial temperature
///  10  solar irradiance placeholder
///  11  wind speed magnitude
///  12  wind direction (degrees)
///  13  sun elevation (degrees)
///  14  ambient temperature
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

    let wind_rad = wind_dir.to_radians();
    let wu = (wind_speed * wind_rad.cos()) as f32;
    let wv = (wind_speed * wind_rad.sin()) as f32;

    for ix in 0..nx {
        for iy in 0..ny {
            for iz in 0..nz {
                tensor[[0, 6, ix, iy, iz]] = wu;
                tensor[[0, 7, ix, iy, iz]] = wv;
                tensor[[0, 9, ix, iy, iz]] = t_ambient as f32;
                tensor[[0, 11, ix, iy, iz]] = wind_speed as f32;
                tensor[[0, 12, ix, iy, iz]] = wind_dir as f32;
                tensor[[0, 13, ix, iy, iz]] = sun_elevation as f32;
                tensor[[0, 14, ix, iy, iz]] = t_ambient as f32;
            }
        }
    }

    for block in geometry {
        let gx = (block.x as i64).clamp(0, (nx as i64) - 1) as usize;
        let gy = (block.y as i64).clamp(0, (ny as i64) - 1) as usize;
        let gz = (block.z as i64).clamp(0, (nz as i64) - 1) as usize;

        let (albedo, emissivity, conductivity, roughness, is_veg) =
            surface_properties(&block.surface_type);

        tensor[[0, 0, gx, gy, gz]] = 1.0;
        tensor[[0, 1, gx, gy, gz]] = albedo;
        tensor[[0, 2, gx, gy, gz]] = emissivity;
        tensor[[0, 3, gx, gy, gz]] = conductivity;
        tensor[[0, 4, gx, gy, gz]] = roughness;
        tensor[[0, 5, gx, gy, gz]] = is_veg;
        tensor[[0, 6, gx, gy, gz]] = 0.0;
        tensor[[0, 7, gx, gy, gz]] = 0.0;
        tensor[[0, 8, gx, gy, gz]] = 0.0;
    }

    tensor
}

/// (albedo, emissivity, thermal_conductivity, roughness, vegetation_flag)
fn surface_properties(surface_type: &str) -> (f32, f32, f32, f32, f32) {
    match surface_type {
        "bitume" | "asphalt" => (0.1, 0.95, 0.75, 0.01, 0.0),
        "herbe" | "grass" | "vegetation" => (0.25, 0.95, 0.3, 0.1, 1.0),
        "eau" | "water" => (0.06, 0.96, 0.6, 0.0001, 0.0),
        "gravier" | "gravel" => (0.2, 0.90, 0.7, 0.015, 0.0),
        "batiment" | "building" | "concrete" => (0.3, 0.90, 1.4, 0.02, 0.0),
        "glass" => (0.4, 0.84, 1.0, 0.001, 0.0),
        "metal" => (0.6, 0.20, 50.0, 0.005, 0.0),
        "brick" => (0.3, 0.90, 0.7, 0.03, 0.0),
        _ => (0.3, 0.90, 1.0, 0.02, 0.0),
    }
}
