//! Kernel cache stub for repeated tensor op compilation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum KernelOp {
    MatMul { m: u32, k: u32, n: u32 },
    Fft1D { len: u32 },
}

#[derive(Debug, Clone)]
pub struct CachedKernel {
    pub op: KernelOp,
    pub hits: u64,
}

#[derive(Default)]
pub struct KernelCache {
    entries: HashMap<KernelOp, CachedKernel>,
}

impl KernelCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_insert(&mut self, op: KernelOp) -> CachedKernel {
        self.entries
            .entry(op.clone())
            .or_insert_with(|| CachedKernel { op, hits: 0 })
            .clone()
    }

    pub fn record_hit(&mut self, op: &KernelOp) {
        if let Some(entry) = self.entries.get_mut(op) {
            entry.hits += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Global kernel cache (Phase 2 stub — LLVM JIT hook planned).
static GLOBAL_CACHE: std::sync::OnceLock<Arc<Mutex<KernelCache>>> = std::sync::OnceLock::new();

pub fn global_kernel_cache() -> Arc<Mutex<KernelCache>> {
    GLOBAL_CACHE
        .get_or_init(|| Arc::new(Mutex::new(KernelCache::new())))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_tracks_ops() {
        let mut c = KernelCache::new();
        let k = c.get_or_insert(KernelOp::MatMul { m: 4, k: 4, n: 4 });
        assert_eq!(k.hits, 0);
        c.record_hit(&KernelOp::MatMul { m: 4, k: 4, n: 4 });
        assert_eq!(c.len(), 1);
    }
}
