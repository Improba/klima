//! Bâtiments OSM (Overpass) → voxels « batiment » dans le tenseur d’entrée.

use ndarray::Array5;
use serde::Deserialize;

use crate::error::AppError;
use crate::inference::overlay_geo::{
    self, GRID_NX, GRID_NY, GRID_NZ, Z_METERS_PER_VOXEL,
};

const MAX_WAYS: usize = 350;
const MAX_VOXELS: usize = 25_000;

#[derive(Debug, Clone)]
pub struct OsmBuildingWay {
    pub ring_lon_lat: Vec<(f64, f64)>,
    pub height_m: f32,
}

#[derive(Deserialize)]
struct OverpassResp {
    elements: Option<Vec<OverpassEl>>,
}

#[derive(Deserialize)]
struct OverpassEl {
    #[serde(rename = "type")]
    el_type: String,
    tags: Option<serde_json::Value>,
    geometry: Option<Vec<OverpassNode>>,
}

#[derive(Deserialize)]
struct OverpassNode {
    lat: f64,
    lon: f64,
}

fn parse_height_m(tags: &serde_json::Value) -> f32 {
    let Some(o) = tags.as_object() else {
        return 12.0;
    };
    if let Some(s) = o
        .get("height")
        .or_else(|| o.get("building:height"))
        .and_then(|v| v.as_str())
    {
        let t = s.trim().replace(',', ".");
        if let Ok(v) = t.parse::<f32>() {
            return v.clamp(4.0, 120.0);
        }
    }
    if let Some(lv) = o
        .get("building:levels")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f32>().ok())
    {
        return (lv * 3.5).clamp(4.0, 120.0);
    }
    12.0
}

/// `west`, `south`, `east`, `north` en degrés WGS84.
pub async fn fetch_building_ways(
    client: &reqwest::Client,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
) -> Result<Vec<OsmBuildingWay>, AppError> {
    let url = std::env::var("KLIMA_OVERPASS_URL")
        .unwrap_or_else(|_| "https://overpass-api.de/api/interpreter".to_string());
    let q = format!(
        "[out:json][timeout:25];
(
  way[\"building\"]({south},{west},{north},{east});
);
out geom;"
    );
    let body = serde_urlencoded::to_string([("data", q.as_str())])
        .map_err(|e| AppError::Internal(format!("Overpass encode: {e}")))?;
    let resp = client
        .post(&url)
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(body)
        .timeout(std::time::Duration::from_secs(35))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Overpass request: {e}")))?;
    if !resp.status().is_success() {
        return Err(AppError::Internal(format!(
            "Overpass HTTP {}",
            resp.status()
        )));
    }
    let body: OverpassResp = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Overpass JSON: {e}")))?;
    let mut out = Vec::new();
    for el in body.elements.unwrap_or_default() {
        if el.el_type != "way" {
            continue;
        }
        let Some(geom) = el.geometry else { continue };
        if geom.len() < 3 {
            continue;
        }
        let ring: Vec<(f64, f64)> = geom.iter().map(|n| (n.lon, n.lat)).collect();
        let h = el
            .tags
            .as_ref()
            .map(parse_height_m)
            .unwrap_or(12.0);
        out.push(OsmBuildingWay {
            ring_lon_lat: ring,
            height_m: h,
        });
        if out.len() >= MAX_WAYS {
            break;
        }
    }
    Ok(out)
}

fn point_in_polygon(px: f64, py: f64, poly: &[(f64, f64)]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if ((yi > py) != (yj > py))
            && (px < (xj - xi) * (py - yi) / (yj - yi + f64::EPSILON) + xi)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn ring_to_grid_xy(ring: &[(f64, f64)]) -> Vec<(f64, f64)> {
    ring.iter()
        .map(|(lon, lat)| {
            let gx = (lon - overlay_geo::OVERLAY_ORIGIN_LON) / overlay_geo::OVERLAY_CELL_DEG;
            let gy = (lat - overlay_geo::OVERLAY_ORIGIN_LAT) / overlay_geo::OVERLAY_CELL_DEG;
            (gx, gy)
        })
        .collect()
}

/// Canal batiment : index 5 → one-hot channel 6 (1..=6 → bitume..batiment).
fn paint_batiment_voxel(tensor: &mut Array5<f32>, ix: usize, iy: usize, iz: usize) {
    tensor[[0, 0, ix, iy, iz]] = 1.0;
    for c in 1..=6 {
        tensor[[0, c, ix, iy, iz]] = 0.0;
    }
    tensor[[0, 6, ix, iy, iz]] = 1.0;
    tensor[[0, 7, ix, iy, iz]] = 0.3;
    tensor[[0, 8, ix, iy, iz]] = 0.90;
    tensor[[0, 9, ix, iy, iz]] = 0.02;
}

pub fn rasterize_ways_into_tensor(tensor: &mut Array5<f32>, ways: &[OsmBuildingWay]) -> usize {
    let mut count = 0usize;
    for way in ways {
        if count >= MAX_VOXELS {
            break;
        }
        let grid_ring = ring_to_grid_xy(&way.ring_lon_lat);
        if grid_ring.len() < 3 {
            continue;
        }
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        for (gx, gy) in &grid_ring {
            min_x = min_x.min(*gx);
            max_x = max_x.max(*gx);
            min_y = min_y.min(*gy);
            max_y = max_y.max(*gy);
        }
        let ix0 = min_x.floor().max(0.0) as i32;
        let ix1 = max_x.ceil().min((GRID_NX - 1) as f64) as i32;
        let iy0 = min_y.floor().max(0.0) as i32;
        let iy1 = max_y.ceil().min((GRID_NY - 1) as f64) as i32;
        if ix0 > ix1 || iy0 > iy1 {
            continue;
        }
        let layers = ((f64::from(way.height_m) / Z_METERS_PER_VOXEL).ceil() as usize).clamp(1, GRID_NZ);
        for ix in ix0..=ix1 {
            if count >= MAX_VOXELS {
                break;
            }
            let iux = ix.clamp(0, (GRID_NX - 1) as i32) as usize;
            for iy in iy0..=iy1 {
                if count >= MAX_VOXELS {
                    break;
                }
                let iuy = iy.clamp(0, (GRID_NY - 1) as i32) as usize;
                let cx = ix as f64 + 0.5;
                let cy = iy as f64 + 0.5;
                if !point_in_polygon(cx, cy, &grid_ring) {
                    continue;
                }
                for iz in 0..layers {
                    if count >= MAX_VOXELS {
                        break;
                    }
                    let iuz = iz.min(GRID_NZ - 1);
                    paint_batiment_voxel(tensor, iux, iuy, iuz);
                    count += 1;
                }
            }
        }
    }
    count
}
