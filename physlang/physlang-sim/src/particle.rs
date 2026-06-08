use crate::traits::{SimError, Simulator};

#[derive(Debug, Default)]
pub struct ParticleStub {
    pub time: f64,
    pub num_particles: usize,
}

impl Simulator for ParticleStub {
    fn step(&mut self, dt: f64) -> Result<(), SimError> {
        self.time += dt;
        Ok(())
    }

    fn time(&self) -> f64 {
        self.time
    }
}
