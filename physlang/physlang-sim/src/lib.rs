//! PhysicsSim — trait definitions for future simulation domains.

mod continuum;
mod em;
mod montecarlo;
mod particle;
mod traits;

pub use continuum::ContinuumStub;
pub use em::EmStub;
pub use montecarlo::MonteCarloStub;
pub use particle::ParticleStub;
pub use traits::{BoundaryCondition, Mesh, SimError, Simulator, Solver};
