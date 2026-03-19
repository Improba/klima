use ndarray::ArrayD;
use serde::Serialize;

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
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationResult {
    pub surface_temperatures: Vec<SurfaceTemperature>,
    pub wind_field: Vec<WindFieldSample>,
    pub metadata: ResultMetadata,
}

/// Extract human-readable results from the raw model output tensor.
///
/// Expected shape: `[1, C, X, Y, Z]` with C >= 4.
///   channel 0 : ΔT  (temperature offset from ambient)
///   channels 1-3 : vx, vy, vz (wind velocity)
pub fn postprocess(output_tensor: &ArrayD<f32>, t_ambient: f64) -> SimulationResult {
    let shape = output_tensor.shape();
    let (nx, ny, nz) = if shape.len() == 5 {
        (shape[2], shape[3], shape[4])
    } else {
        (64, 64, 16)
    };
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

    SimulationResult {
        metadata: ResultMetadata {
            grid_resolution: [nx, ny, nz],
            wind_subsample: [target_wx.min(nx), target_wy.min(ny), target_wz.min(nz)],
            num_surface_points: surface_temperatures.len(),
            num_wind_samples: wind_field.len(),
        },
        surface_temperatures,
        wind_field,
    }
}
