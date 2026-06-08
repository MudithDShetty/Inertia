use crate::traits::{SimError, Simulator};

#[derive(Debug, Default)]
pub struct MonteCarloStub {
    pub samples: u64,
    pub estimate: f64,
}

impl Simulator for MonteCarloStub {
    fn step(&mut self, dt: f64) -> Result<(), SimError> {
        let _ = dt;
        self.samples += 1;
        Ok(())
    }

    fn time(&self) -> f64 {
        self.samples as f64
    }
}

impl MonteCarloStub {
    pub fn accumulate(&mut self, sample: f64) {
        let n = self.samples as f64;
        self.estimate += (sample - self.estimate) / n.max(1.0);
    }
}
