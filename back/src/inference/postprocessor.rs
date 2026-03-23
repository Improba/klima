use ndarray::ArrayD;
use serde::Serialize;

use crate::inference::overlay_geo;

/// Occupancy sous-échantillonnée (canal 0 du tenseur d’entrée), pour le client : éviter les grosses charges JSON.
#[derive(Debug, Clone, Serialize)]
pub struct OccupancyGrid {
    /// Facteur par rapport à `metadata.grid_resolution` (ex. 8 → 256→32).
    pub stride: [usize; 3],
    pub dims: [usize; 3],
    /// Row-major indexation : `ix + dims[0] * (iy + dims[1] * iz)`, 0 = air, 1 = solide.
    pub cells: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurfaceTemperature {
    pub lon: f64,
    pub lat: f64,
    pub alt: f64,
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindFieldSample {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultMetadata {
    pub grid_resolution: [usize; 3],
    pub wind_subsample: [usize; 3],
    pub num_surface_points: usize,
    pub num_wind_samples: usize,
    pub inference_time_ms: u64,
    pub model_loaded: bool,
    pub t_ambient: f64,
    pub delta_t_range: [f64; 2],
    pub wind_speed_range: [f64; 2],
    /// Hauteur verticale d’une couche (m), alignée sur `overlay_geo::Z_METERS_PER_VOXEL`.
    pub z_meters_per_voxel: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationResult {
    pub surface_temperatures: Vec<SurfaceTemperature>,
    pub wind_field: Vec<WindFieldSample>,
    pub metadata: ResultMetadata,
    /// Présent si le tenseur d’entrée était disponible (toujours en prod après preprocess).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occupancy: Option<OccupancyGrid>,
}

fn pack_occupancy_cells(
    nx: usize,
    ny: usize,
    nz: usize,
    stride: [usize; 3],
    mut occ: impl FnMut(usize, usize, usize) -> f32,
) -> OccupancyGrid {
    let sx = stride[0].max(1);
    let sy = stride[1].max(1);
    let sz = stride[2].max(1);
    let dx = (nx + sx - 1) / sx;
    let dy = (ny + sy - 1) / sy;
    let dz = (nz + sz - 1) / sz;
    let mut cells = Vec::with_capacity(dx * dy * dz);
    for iz in 0..dz {
        let z0 = iz * sz;
        let z1 = ((iz + 1) * sz).min(nz);
        for iy in 0..dy {
            let y0 = iy * sy;
            let y1 = ((iy + 1) * sy).min(ny);
            for ix in 0..dx {
                let x0 = ix * sx;
                let x1 = ((ix + 1) * sx).min(nx);
                let mut mx = 0f32;
                for k in z0..z1 {
                    for j in y0..y1 {
                        for i in x0..x1 {
                            let v = occ(i, j, k);
                            if v > mx {
                                mx = v;
                            }
                        }
                    }
                }
                cells.push(if mx > 0.5 { 1 } else { 0 });
            }
        }
    }
    OccupancyGrid {
        stride: [sx, sy, sz],
        dims: [dx, dy, dz],
        cells,
    }
}

/// Sous-échantillonne le canal occupancy (max-pool > 0.5 → solide).
pub fn pack_occupancy(input: &ArrayD<f32>, stride: [usize; 3]) -> Option<OccupancyGrid> {
    let s = input.shape();
    if s.len() != 5 || s[0] < 1 || s[1] < 1 {
        return None;
    }
    let nx = s[2];
    let ny = s[3];
    let nz = s[4];
    Some(pack_occupancy_cells(nx, ny, nz, stride, |i, j, k| input[[0, 0, i, j, k]]))
}

/// Extract human-readable results from the raw model output tensor.
///
/// Expected shape: `[1, C, X, Y, Z]` with C >= 4.
///   channel 0 : ΔT  (temperature offset from ambient)
///   channels 1-3 : vx, vy, vz (wind velocity)
///
/// `occupancy` : grille d’occupancy sous-échantillonnée (voir `pack_occupancy`), produite à partir du tenseur d’entrée.
pub fn postprocess(
    output_tensor: &ArrayD<f32>,
    occupancy: Option<OccupancyGrid>,
    t_ambient: f64,
    inference_time_ms: u64,
    model_loaded: bool,
) -> SimulationResult {
    let shape = output_tensor.shape();
    let (nx, ny, nz) = if shape.len() == 5 {
        (shape[2], shape[3], shape[4])
    } else {
        (64, 64, 16)
    };

    let z_mpv = overlay_geo::Z_METERS_PER_VOXEL;

    if nx == 0 || ny == 0 || nz == 0 {
        return SimulationResult {
            surface_temperatures: vec![],
            wind_field: vec![],
            metadata: ResultMetadata {
                grid_resolution: [nx, ny, nz],
                wind_subsample: [0, 0, 0],
                num_surface_points: 0,
                num_wind_samples: 0,
                inference_time_ms,
                model_loaded,
                t_ambient,
                delta_t_range: [0.0, 0.0],
                wind_speed_range: [0.0, 0.0],
                z_meters_per_voxel: z_mpv,
            },
            occupancy,
        };
    }

    let has_channels = shape.len() == 5 && shape[1] >= 4;

    let mut surface_temperatures = Vec::with_capacity(nx * ny);
    for ix in 0..nx {
        for iy in 0..ny {
            let dt = if has_channels {
                output_tensor[[0, 0, ix, iy, 0]] as f64
            } else {
                0.0
            };
            surface_temperatures.push(SurfaceTemperature {
                lon: ix as f64,
                lat: iy as f64,
                alt: 0.0,
                temperature: t_ambient + dt,
            });
        }
    }

    let (target_wx, target_wy, target_wz) = (64usize, 64usize, 16usize);
    let step_x = (nx / target_wx.min(nx)).max(1);
    let step_y = (ny / target_wy.min(ny)).max(1);
    let step_z = (nz / target_wz.min(nz)).max(1);

    let mut wind_field = Vec::new();
    let mut ix = 0;
    while ix < nx {
        let mut iy = 0;
        while iy < ny {
            let mut iz = 0;
            while iz < nz {
                let (vx, vy, vz) = if has_channels {
                    (
                        output_tensor[[0, 1, ix, iy, iz]] as f64,
                        output_tensor[[0, 2, ix, iy, iz]] as f64,
                        output_tensor[[0, 3, ix, iy, iz]] as f64,
                    )
                } else {
                    (0.0, 0.0, 0.0)
                };
                wind_field.push(WindFieldSample {
                    x: ix as f64,
                    y: iy as f64,
                    z: iz as f64,
                    vx,
                    vy,
                    vz,
                });
                iz += step_z;
            }
            iy += step_y;
        }
        ix += step_x;
    }

    let delta_t_range = if surface_temperatures.is_empty() {
        [0.0, 0.0]
    } else {
        let mut min_dt = f64::MAX;
        let mut max_dt = f64::MIN;
        for st in &surface_temperatures {
            let dt = st.temperature - t_ambient;
            if dt < min_dt { min_dt = dt; }
            if dt > max_dt { max_dt = dt; }
        }
        [min_dt, max_dt]
    };

    let wind_speed_range = if wind_field.is_empty() {
        [0.0, 0.0]
    } else {
        let mut min_ws = f64::MAX;
        let mut max_ws = f64::MIN;
        for w in &wind_field {
            let speed = (w.vx * w.vx + w.vy * w.vy + w.vz * w.vz).sqrt();
            if speed < min_ws { min_ws = speed; }
            if speed > max_ws { max_ws = speed; }
        }
        [min_ws, max_ws]
    };

    SimulationResult {
        metadata: ResultMetadata {
            grid_resolution: [nx, ny, nz],
            wind_subsample: [target_wx.min(nx), target_wy.min(ny), target_wz.min(nz)],
            num_surface_points: surface_temperatures.len(),
            num_wind_samples: wind_field.len(),
            inference_time_ms,
            model_loaded,
            t_ambient,
            delta_t_range,
            wind_speed_range,
            z_meters_per_voxel: z_mpv,
        },
        surface_temperatures,
        wind_field,
        occupancy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array5;

    #[test]
    fn postprocess_reads_delta_t_at_ground_slice() {
        let mut t = Array5::<f32>::zeros((1, 4, 4, 4, 4));
        t[[0, 0, 1, 2, 0]] = 3.5;
        let r = postprocess(&t.into_dyn(), None, 20.0, 42, true);
        assert_eq!(r.surface_temperatures.len(), 16);
        let st = &r.surface_temperatures[1 * 4 + 2];
        assert!((st.lon - 1.0).abs() < 1e-9);
        assert!((st.lat - 2.0).abs() < 1e-9);
        assert!((st.temperature - 23.5).abs() < 1e-5);
        assert_eq!(r.metadata.inference_time_ms, 42);
        assert!(r.metadata.model_loaded);
    }

    #[test]
    fn postprocess_empty_shape_returns_empty_vectors() {
        let t = Array5::<f32>::zeros((1, 4, 4, 0, 4));
        let r = postprocess(&t.into_dyn(), None, 10.0, 0, false);
        assert!(r.surface_temperatures.is_empty());
        assert!(r.wind_field.is_empty());
    }
}
