pub mod arrowed_place_holder;
pub mod dictionary;
pub mod grid;
pub mod solver;

// Re-export key types at crate root for cleaner imports
pub use arrowed_place_holder::{ArrowedPlaceHolder, ARROWED_PLACE_HOLDERS};
pub use dictionary::FlatTrie;
pub use grid::{Board, Capelito, CrosswordGrid, GridError};
pub use solver::solve_backtrack;
