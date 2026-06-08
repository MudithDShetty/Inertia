use crate::traits::{BoundaryCondition, Mesh, SimError, Simulator, Solver};
use physlang_math::Tensor;

#[derive(Debug, Default)]
pub struct ContinuumStub {
    pub time: f64,
    pub nodes: usize,
    pub cells: usize,
}

impl Simulator for ContinuumStub {
    fn step(&mut self, dt: f64) -> Result<(), SimError> {
        self.time += dt;
        Ok(())
    }

    fn time(&self) -> f64 {
        self.time
    }
}

impl Mesh for ContinuumStub {
    fn num_nodes(&self) -> usize {
        self.nodes
    }

    fn num_cells(&self) -> usize {
        self.cells
    }

    fn dimension(&self) -> usize {
        3
    }
}

impl Solver for ContinuumStub {
    fn solve(&mut self) -> Result<(), SimError> {
        Ok(())
    }

    fn residual_norm(&self) -> f64 {
        0.0
    }
}

pub struct DirichletStub {
    pub name: String,
    pub value: f64,
}

impl BoundaryCondition for DirichletStub {
    fn name(&self) -> &str {
        &self.name
    }

    fn apply(&self, field: &mut Tensor) -> Result<(), SimError> {
        for v in field.data_mut() {
            *v = self.value;
        }
        Ok(())
    }
}
