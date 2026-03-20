pub mod fno_client;
pub mod postprocessor;
pub mod preprocessor;

use tokio::sync::Mutex;

use ndarray::{ArrayD, Axis};
use ort::session::Session;
use ort::value::TensorRef;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct NormParams {
    pub input_mean: Vec<f32>,
    pub input_std: Vec<f32>,
    pub output_mean: Vec<f32>,
    pub output_std: Vec<f32>,
}

pub struct OnnxService {
    session: Option<Mutex<Session>>,
    norm_params: Option<NormParams>,
}

impl OnnxService {
    pub fn new(model_path: Option<&str>, norm_path: Option<&str>) -> Self {
        let session = model_path.and_then(|path| {
            match Session::builder().and_then(|mut b| b.commit_from_file(path)) {
                Ok(s) => {
                    tracing::info!("ONNX model loaded from {}", path);
                    Some(Mutex::new(s))
                }
                Err(e) => {
                    tracing::warn!("Failed to load ONNX model: {}", e);
                    None
                }
            }
        });

        let norm_path = norm_path.filter(|p| !p.is_empty());
        let norm_explicit = norm_path.is_some();

        let norm_params = norm_path.and_then(|path| {
            match std::fs::read_to_string(path) {
                Ok(c) => match serde_json::from_str::<NormParams>(&c) {
                    Ok(p) => Some(p),
                    Err(e) => {
                        tracing::warn!(
                            "Invalid norm_params.json at {}: {} — inference will skip normalization for those stats",
                            path,
                            e
                        );
                        None
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        "Could not read norm params at {}: {} — inference will skip normalization for those stats",
                        path,
                        e
                    );
                    None
                }
            }
        });

        let out = Self {
            session,
            norm_params,
        };
        if out.session.is_some() && out.norm_params.is_none() && !norm_explicit {
            tracing::warn!(
                "ONNX model loaded without KLIMA_NORM_PATH — set it for channel-wise normalization (recommended for trained checkpoints)"
            );
        }
        out
    }

    pub fn is_loaded(&self) -> bool {
        self.session.is_some()
    }

    pub async fn predict(&self, input: ArrayD<f32>) -> Result<ArrayD<f32>, AppError> {
        let session_mutex = self
            .session
            .as_ref()
            .ok_or_else(|| AppError::Internal("ONNX model not loaded".into()))?;

        let mut session = session_mutex.lock().await;

        let normalized = self.normalize_input(input);

        let input_ref = TensorRef::from_array_view(&normalized)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let outputs = session
            .run(ort::inputs![input_ref])
            .map_err(|e| AppError::Internal(format!("Inference failed: {}", e)))?;

        let output_view = outputs[0]
            .try_extract_array::<f32>()
            .map_err(|e| AppError::Internal(format!("Output extraction failed: {}", e)))?;

        let result = output_view.to_owned();
        Ok(self.denormalize_output(result))
    }

    fn normalize_input(&self, mut input: ArrayD<f32>) -> ArrayD<f32> {
        if let Some(params) = &self.norm_params {
            let nch = input.shape().get(1).copied().unwrap_or(0);
            for c in 0..nch.min(params.input_mean.len()) {
                let std = params.input_std[c].max(1e-8);
                let mean = params.input_mean[c];
                input
                    .index_axis_mut(Axis(1), c)
                    .mapv_inplace(|v| (v - mean) / std);
            }
        }
        input
    }

    fn denormalize_output(&self, mut output: ArrayD<f32>) -> ArrayD<f32> {
        if let Some(params) = &self.norm_params {
            let nch = output.shape().get(1).copied().unwrap_or(0);
            for c in 0..nch.min(params.output_mean.len()) {
                let std = params.output_std[c];
                let mean = params.output_mean[c];
                output
                    .index_axis_mut(Axis(1), c)
                    .mapv_inplace(|v| v * std + mean);
            }
        }
        output
    }
}
