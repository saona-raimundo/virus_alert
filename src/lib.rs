//! Virus alert simulation package.
//!
//! This crate allows to simulate and study the dynamics defined in the
//! [Virus Alert](https://ist.ac.at/en/education/ist-for-kids/virus-alert/) educational board game.
//!

pub use building::{Building, BuildingBuilder};
pub use individual::Individual;
pub use population::Population;
pub use board::Board;
pub use recording::Recording;
pub use simulation::{Simulation, SimulationBuilder};

/// Individuals that can be in different states of health.
pub mod individual;
/// Buildings which individuals visit.
pub mod building;
/// Aggregate of individuals. 
pub mod population; 
/// Aggregate of buildings and population.
pub mod board;
/// Resources used to keep track of the state of the game.
pub mod recording;
/// Simulation setup and results.
pub mod simulation;

/// All you should need to play the game. 
pub mod prelude {
	pub use crate::{
        simulation::Report,
        simulation::report::ReportPlan, 
        Board, 
        Individual, 
        Population, 
        board::BoardBuilder, 
        Simulation, 
        SimulationBuilder,
        building::Spreading,
    };
}

/// All errors in this crate.
pub mod errors {
    use thiserror::Error;

    #[derive(Error, Debug, PartialEq, Eq)]
    pub enum BuildingError {
        #[error("building is full")]
        Full,
        #[error("Sick individuals are not allowed in the buildings")]
        Sick,
    }

    #[derive(Error, Debug, PartialEq, Eq)]
    pub enum ActionError {
        #[error("There are no more healthy individuals in the population")]
        NoHealthyLeft,
        #[error("There are no more immune individuals in the population")]
        NoImmuneLeft,
    }
}

#[cfg(test)]
mod tests {
    /// Construct a deterministic RNG with the given seed
    pub fn rng(seed: u64) -> impl rand::RngCore {
        // For tests, we want a statistically good, fast, reproducible RNG.
        // PCG32 will do fine, and will be easy to embed if we ever need to.
        const INC: u64 = 11634580027462260723;
        rand_pcg::Pcg32::new(seed, INC)
    }
}
