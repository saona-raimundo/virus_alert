use crate::recording::CountingTable;
use core::fmt::Display;
use crate::{BuildingBuilder, Building, Population, Individual, Recording, building::Spreading};
use getset::{Getters, Setters, MutGetters};
use serde::{Serialize, Deserialize};

/// Builder for the `Board`.
///
/// # Remarks
///
/// Although `Board` can be constructed from `new` and `set_spreading`, this 
/// struct is specifically thought to be serialized and deserialized in a human-frindly way,
/// specially useful as a configuration file.
///   
/// A `Board` could be in the middle of a game, derefore (de)serialization 
/// turns out to be less human-friendly.
#[derive(Debug, Clone, PartialEq, Eq, Getters, Setters, MutGetters, Serialize, Deserialize, Default)]
pub struct BoardBuilder {
	/// Number of healthy individuals
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    pub healthy: usize,
    /// Number of infected1 individuals
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    pub infected1: usize,
    /// Number of infected2 individuals
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    pub infected2: usize,
    /// Number of infected3 individuals
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    pub infected3: usize,
    /// Number of sick individuals
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    pub sick: usize,
    /// Number of immune individuals
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    pub immune: usize,
    /// Current state of the buildings in the game
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    pub buildings: Vec<(usize, usize)>,
    /// Spreading mode
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    pub spreading: Spreading,
}

impl BoardBuilder {
	pub fn build(self) -> Board {
		// Population
		let mut population_vec = vec![Individual::Healthy; self.healthy];
		population_vec.append(&mut vec![Individual::Infected1; self.infected1]);
		population_vec.append(&mut vec![Individual::Infected2; self.infected2]);
		population_vec.append(&mut vec![Individual::Infected3; self.infected3]);
		population_vec.append(&mut vec![Individual::Sick; self.sick]);
		population_vec.append(&mut vec![Individual::Immune; self.immune]);
		let population = Population::from(population_vec);

		// Buildings
		let buildings = self.buildings.iter().map(|&(cols, rows)| 
			BuildingBuilder::new("Defult")
				.with_size(cols, rows)
				.with_spreading(self.spreading)
				.and_is_open()
				.build()
			).collect();

		Board::new(population, buildings)
	}
}


/// Represents the state of the game and have high level commands.
#[derive(Debug, Clone, PartialEq, Eq, Getters, MutGetters)]
pub struct Board {
	/// Current population in the game
    #[getset(get = "pub", get_mut)]
    population: Population,
    /// Current state of the buildings in the game
    #[getset(get = "pub")]
    buildings: Vec<Building>,
    inactive: Vec<Individual>, 
    /// Recording device
    #[getset(get = "pub", get_mut)]
    recording: Recording,
}

impl Board {
	/// Creates a new board with the specified population and buildings as default.
	///
	/// # Panics
	///
	/// If not all buildings have the same spreading mode.
	pub fn new(population: Population, buildings: Vec<Building>) -> Self {
		assert_eq!(
			buildings.iter().map(|b| b.spreading()).min(), 
			buildings.iter().map(|b| b.spreading()).max()
		);
		let default = Board::default();
		let recording = Recording::new(population.clone(), buildings.clone());
		Board {
			population,
			buildings,
			recording,
			..default
		}
	}

	/// Immunize one person in the population. 
	/// 
	/// # Errors
	///
	/// If there is no healthy individual to immunize.
	///
	/// # Examples
	///
	/// Immunize one person from the default population.
	/// ```
	/// # use virus_alarm::prelude::*;
	/// let mut board = Board::default();
	/// board.immunize();
	/// assert_eq!(board.population().counting(Individual::Immune), 1);
	/// ```
	pub fn immunize(&mut self) -> Result<&mut Self, crate::errors::ActionError> {
		self.population_mut().immunize()?;
		self.recording_mut().immunize()?;
		Ok(self)
	}

	/// Reverse one individual from immune to healthy in the population. 
	/// 
	/// # Errors
	///
	/// If there is no immune individual to reverse.
	///
	/// # Examples
	///
	/// Immunize one person from the default population and then set it back.
	/// ```
	/// # use virus_alarm::prelude::*;
	/// let mut board = Board::default();
	/// board.immunize();
	/// assert_eq!(board.population().counting(Individual::Immune), 1);
	/// board.reverse_immunize();
	/// assert_eq!(board.population().counting(Individual::Immune), 0);
	/// ```
	pub fn reverse_immunize(&mut self) -> Result<&mut Self, crate::errors::ActionError> {
		self.population_mut().reverse_immunize()?;
		self.recording_mut().reverse_immunize()?;
		Ok(self)
	}

	/// Advance the specified number of stages in the game.
	///
	/// # Remarks
	///
	/// This is equivalent to use `advance` many times.
	pub fn advance_many(&mut self, num_stages: usize) -> &mut Self{
		for _ in 0..num_stages {
			self.advance();
		}
		self
	}

	/// Advance the population a stage in the game, without registering the changes.
	///
	/// Returns the number of newly infected individuals
	pub fn advance_population(&mut self) -> usize {
		self.visit();
		self.propagate();
		self.go_home()
	}


	/// Advance a stage in the game.
	///
	/// # Remarks
	///
	/// This is a short method for all steps involved in a stage.
	pub fn advance(&mut self) -> &mut Self {
		let newly_infected = self.advance_population();
		self.recording.register(newly_infected, &self.buildings);
		self
	}

	/// First step of any stage
	///
	/// In this step, buildings are populated by non-sick individuals randomly.
	///
	/// # Errors
	///
	/// If visiting any of the building fails.
	pub fn visit(&mut self) -> &mut Self {
		// Randomness
		self.population.shuffle(&mut rand::thread_rng());
		// Visiting
		for index in 0..self.buildings.len() {
			self.visit_building(index);
		}
		// Remaining individuals are stored in inactive 
		self.inactive.extend(self.population.clone()); 
		self
	}

	fn visit_building(&mut self, index: usize) -> &Building {
		while !self.buildings[index].is_full() & self.buildings[index].is_open() {
			match self.population.next() {
				Some(i) => {
					match i {
						Individual::Sick => self.inactive.push(i),
						i => self.buildings[index].try_push(i).expect("pushing on a building with space failed!"),
					}
				},
				None => break,
			}
		}
		&self.buildings[index]
	}

	/// Second step of any stage
	///
	/// In this step, virus is propagated in each building.
	pub fn propagate(&mut self) {
		// Buildings
		for building in self.buildings.iter_mut() {
			building.propagate();
		}
		// Inactive
		for i in self.inactive.iter_mut() {
			*i = match i {
				Individual::Infected1 => Individual::Infected2,
				Individual::Infected2 => Individual::Infected3,
				Individual::Infected3 => Individual::Sick,
				_ => *i,
			}
		}
	}

	/// Third step of any stage
	///
	/// In this step, the population returns home. 
	/// Outputs the number of newly infected.
	pub fn go_home(&mut self) -> usize {
		let mut new_vec = Vec::new();
		// Collect 
		// From buildings
		for building in self.buildings.iter_mut() {
			new_vec.append(&mut building.empty())
		}
		let newly_infected: usize = new_vec.iter().filter(|&&i| i == Individual::Infected1).count();
		// From inactive
		new_vec.append(&mut self.inactive);
		let new_population = Population::from(new_vec);

		// Update
		self.population = new_population;

		newly_infected
	}


	/// Closes a building
	pub fn toggle<S: Display>(&mut self, name: S) -> &mut Self {
		for building in self.buildings.iter_mut() {
			if building.name() == name.to_string() {
				building.toggle();
			}
		}
		self
	}

	/// Closes a building
	pub fn close<S: Display>(&mut self, name: S) -> &mut Self {
		for building in self.buildings.iter_mut() {
			if building.name() == name.to_string() {
				building.close();
			}
		}
		self
	}

	/// Opens a building
	pub fn open<S: Display>(&mut self, name: S) -> &mut Self {
		for building in self.buildings.iter_mut() {
			if building.name() == name.to_string() {
				building.open();
			}
		}
		self
	}

	/// Returns the spreading mode. 
	///
	/// See `Spreading` for more. 
	///
	/// # Panics
	///
	/// If there are no buildings.
	///
	/// # Examples
	///
	/// Checking the default value.
	/// ```
	/// # use virus_alarm::prelude::*;
	/// let board = Board::default();
	/// assert_eq!(&Spreading::OneNear, board.spreading());
	/// ```
	pub fn spreading(&self) -> &Spreading {
		self.buildings()[0].spreading()
	}

	/// Changes the spreading mode. 
	///
	/// See `Spreading` for more. 
	pub fn set_spreading(&mut self, new_spreading: Spreading) -> &mut Self {
		for building in self.buildings.iter_mut() {
			building.set_spreading(new_spreading);
		}
		self.recording_mut().set_spreading(new_spreading);
		self
	}

	/// Returns the current state of the counting table
	pub fn counting_table(&self) -> &CountingTable {
		self.recording().counting_table()
	}
}

impl Default for Board {
	/// Returns an instance of `Board` with default configuration
	///
	/// # Default
	///
	/// Some default values.
	/// ```
	/// # use virus_alarm::prelude::*;
	/// let board = Board::default();
	/// assert_eq!(board.population().len(), 100);
	/// assert_eq!(board.buildings().len(), 8);
	/// ```
	fn default() -> Self { 
		let population = Population::default();
		let concert_hall = BuildingBuilder::new("Concert Hall").with_size(5, 4).build();
		let bakery = BuildingBuilder::new("Bakery").with_size(2, 2).build();
		let school = BuildingBuilder::new("School").with_size(4, 4).build();
		let pharmacy = BuildingBuilder::new("Pharmacy").with_size(2, 2).build();
		let restaurant = BuildingBuilder::new("Restaurant").with_size(4, 3).build();
		let gym = BuildingBuilder::new("Gym").with_size(4, 2).build();
		let supermarket = BuildingBuilder::new("Supermarket").with_size(2, 2).build();
		let shopping_center = BuildingBuilder::new("Shopping Center").with_size(4, 2).build();
		let buildings = vec![
			concert_hall,
			bakery,
			school,
			pharmacy,
			restaurant,
			gym,
			supermarket,
			shopping_center,
		];
		let recording = Recording::new(population.clone(), buildings.clone());

		Board{ population, buildings, inactive: Vec::new(), recording }
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	use ndarray::array;


	#[test]
	fn visit1() {
		let population = Population::from(vec![Individual::Healthy]);
		let buildings = vec![Building::unchecked_from(array![[None]]), Building::unchecked_from(array![[None]])];
		let default = Board::default();
		let mut board = Board{
			population,
			buildings,
			..default
		};
		board.visit();
		let expected = vec![Building::unchecked_from(array![[Individual::Healthy]]), Building::unchecked_from(array![[None]])];
		assert_eq!(board.buildings(), &expected);
	}

	#[test]
	fn visit2() {
		let population = Population::from(vec![Individual::Infected1, Individual::Infected1]);
		let buildings = vec![Building::unchecked_from(array![[None]])];
		let default = Board::default();
		let mut board = Board{
			population,
			buildings,
			..default
		};
		board.visit();
		let expected = vec![Individual::Infected1];
		assert_eq!(board.inactive, expected);
	}

	#[test]
	fn visit_building1() {
		let population = Population::from(vec![Individual::Healthy]);
		let buildings = vec![Building::unchecked_from(array![[None]])];
		let mut board = Board::new(population, buildings);
		assert_eq!(board.visit_building(0), &Building::unchecked_from(array![[Individual::Healthy]]));
	}

	#[test]
	fn visit_building2() {
		let population = Population::from(vec![Individual::Sick]);
		let buildings = vec![Building::unchecked_from(array![[None]])];
		let mut board = Board::new(population, buildings);
		assert_eq!(board.visit_building(0), &Building::unchecked_from(array![[None]]));
	}

	#[test]
	fn visit_building3() {
		let population = Population::from(vec![Individual::Infected1, Individual::Infected1]);
		let buildings = vec![Building::unchecked_from(array![[Individual::Healthy]])];
		let mut board = Board::new(population, buildings);
		assert_eq!(board.visit_building(0), &Building::unchecked_from(array![[Individual::Healthy]]));
	}

	#[test]
	fn propagate() {
		let population = Population::from(vec![Individual::Infected1, Individual::Infected1]);
		let buildings = vec![Building::unchecked_from(array![[Individual::Healthy, Individual::Infected1]])];
		let mut board = Board::new(population.clone(), buildings);
		board.visit(); // Fills the inactive vector
		board.propagate();
		assert_eq!(board.buildings()[0], Building::unchecked_from(array![[Individual::Infected1, Individual::Infected2]]));
		assert_eq!(board.population(), &population); // All buildings were full so the population was only shuffled!
		assert_eq!(board.inactive, vec![Individual::Infected2, Individual::Infected2]); // Propagation at home!
	}

	#[test]
	fn advance_population1() {
		let population = Population::from(vec![Individual::Healthy, Individual::Sick, Individual::Immune]);
		let buildings = vec![Building::new(2, 2, "My bulding")];
		let mut board = Board::new(population, buildings);
		assert_eq!(board.advance_population(), 0);
	}

	#[test]
	fn advance() {
		let mut board = Board::default();
		for _ in 0..96 {
			board.immunize().unwrap();
		}
		assert_eq!(board.population().counting(Individual::Immune), 96);
		board.advance_many(8);
		for _ in 0..10 {
			board.advance();
			if board.population().counting(Individual::Infected1) != 0 {
				panic!("Recording table:\n{}", board.recording().counting_table());
			}
		}
	}

	#[test]
	#[should_panic]
	fn close() {
		let population = Population::from(vec![Individual::Healthy, Individual::Infected1]);
		let buildings = vec![Building::new(2, 1, "My bulding")];
		let mut board = Board::new(population, buildings);
		board.visit();
		board.close("My bulding");
	}
}