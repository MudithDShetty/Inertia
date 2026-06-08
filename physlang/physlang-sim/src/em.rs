use crate::traits::{SimError, Simulator, Solver};

#[derive(Debug, Default)]
pub struct EmStub {
    pub time: f64,
    pub grid_points: usize,
}

impl Simulator for EmStub {
    fn step(&mut self, dt: f64) -> Result<(), SimError> {
        self.time += dt;
        Ok(())
    }

    fn time(&self) -> f64 {
        self.time
    }
}

impl Solver for EmStub {
    fn solve(&mut self) -> Result<(), SimError> {
        Ok(())
    }

    fn residual_norm(&self) -> f64 {
        0.0
    }
}
