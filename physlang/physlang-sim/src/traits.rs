use physlang_math::Tensor;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum SimError {
    #[error("simulation error: {0}")]
    Message(String),
}

pub trait Simulator {
    fn step(&mut self, dt: f64) -> Result<(), SimError>;
    fn time(&self) -> f64;
}

pub trait Mesh {
    fn num_nodes(&self) -> usize;
    fn num_cells(&self) -> usize;
    fn dimension(&self) -> usize;
}

pub trait BoundaryCondition {
    fn name(&self) -> &str;
    fn apply(&self, field: &mut Tensor) -> Result<(), SimError>;
}

pub trait Solver {
    fn solve(&mut self) -> Result<(), SimError>;
    fn residual_norm(&self) -> f64;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimState {
    pub time: f64,
    pub step: u64,
}
