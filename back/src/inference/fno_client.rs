//! Binary HTTP client for the PyTorch FNO sidecar (`training/infer_server`).
//!
//! Use a shared [`reqwest::Client`] from [`crate::AppState`] (timeouts + connection reuse).
//!
//! Wire format: `KLM1` magic (u32 LE), `ndim` (u32 LE), `ndim` shape values (u32 LE each),
//! then raw `f32` row-major tensor data (C-contiguous, same layout as ndarray).

use ndarray::{ArrayD, ShapeError};

use crate::error::AppError;

const FNO_MAGIC: u32 = 0x314D_4C4B; // b"KLM1" little-endian

fn encode_tensor(input: &ArrayD<f32>) -> Result<Vec<u8>, AppError> {
    let sl = input
        .as_slice_memory_order()
        .ok_or_else(|| AppError::Internal("FNO input tensor must be memory-contiguous".into()))?;
    let shape: Vec<u32> = input
        .shape()
        .iter()
        .map(|&d| u32::try_from(d).map_err(|_| AppError::Internal("shape dim too large".into())))
        .collect::<Result<_, _>>()?;
    let ndim = shape.len() as u32;
    let mut out = Vec::with_capacity(8 + 4 * shape.len() + sl.len() * 4);
    out.extend_from_slice(&FNO_MAGIC.to_le_bytes());
    out.extend_from_slice(&ndim.to_le_bytes());
    for s in &shape {
        out.extend_from_slice(&s.to_le_bytes());
    }
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(sl.as_ptr().cast::<u8>(), std::mem::size_of_val(sl))
    };
    out.extend_from_slice(bytes);
    Ok(out)
}

fn decode_tensor(body: &[u8]) -> Result<ArrayD<f32>, AppError> {
    if body.len() < 8 {
        return Err(AppError::Internal("FNO response too short".into()));
    }
    let magic = u32::from_le_bytes(body[0..4].try_into().unwrap());
    if magic != FNO_MAGIC {
        return Err(AppError::Internal(format!(
            "FNO response bad magic {magic:#x}"
        )));
    }
    let ndim = u32::from_le_bytes(body[4..8].try_into().unwrap()) as usize;
    if ndim == 0 || ndim > 8 {
        return Err(AppError::Internal("FNO response bad ndim".into()));
    }
    let header = 8 + 4 * ndim;
    if body.len() < header {
        return Err(AppError::Internal("FNO response truncated header".into()));
    }
    let mut dims = Vec::with_capacity(ndim);
    for i in 0..ndim {
        let off = 8 + 4 * i;
        dims.push(usize::try_from(u32::from_le_bytes(
            body[off..off + 4].try_into().unwrap(),
        ))
        .map_err(|_| AppError::Internal("FNO shape dim overflow".into()))?);
    }
    let raw = &body[header..];
    if raw.len() % 4 != 0 {
        return Err(AppError::Internal("FNO response payload not f32-aligned".into()));
    }
    let n = raw.len() / 4;
    let expected: usize = dims.iter().product();
    if n != expected {
        return Err(AppError::Internal(format!(
            "FNO response size mismatch: {n} floats vs shape product {expected}"
        )));
    }
    let mut vec = vec![0f32; n];
    for (i, chunk) in raw.chunks_exact(4).enumerate() {
        vec[i] = f32::from_le_bytes(chunk.try_into().unwrap());
    }
    ArrayD::from_shape_vec(dims, vec).map_err(|e: ShapeError| {
        AppError::Internal(format!("FNO output reshape failed: {e}"))
    })
}

/// Run remote FNO inference. `base_url` is e.g. `http://klima-infer:8000` (no trailing slash).
pub async fn predict(
    client: &reqwest::Client,
    base_url: &str,
    input: &ArrayD<f32>,
) -> Result<ArrayD<f32>, AppError> {
    let url = format!("{}/predict", base_url.trim_end_matches('/'));
    let payload = encode_tensor(input)?;

    let resp = client
        .post(&url)
        .header("content-type", "application/octet-stream")
        .body(payload)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("FNO request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "FNO server {status}: {text}"
        )));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| AppError::Internal(format!("FNO read body: {e}")))?;
    decode_tensor(&bytes)
}
