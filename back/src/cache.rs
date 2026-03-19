use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

use crate::inference::postprocessor::SimulationResult;

pub struct SimulationCache {
    inner: Mutex<LruCache<String, SimulationResult>>,
}

impl SimulationCache {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(128).unwrap());
        Self {
            inner: Mutex::new(LruCache::new(cap)),
        }
    }

    pub fn get(&self, key: &str) -> Option<SimulationResult> {
        self.inner.lock().ok()?.get(key).cloned()
    }

    pub fn insert(&self, key: &str, result: SimulationResult) {
        if let Ok(mut cache) = self.inner.lock() {
            cache.put(key.to_string(), result);
        }
    }
}
