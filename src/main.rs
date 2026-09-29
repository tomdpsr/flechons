use std::process;

use clap::{Parser, Subcommand};
use flechons::grid::extract_pattern;
use flechons::{solve_backtrack, Capelito, CrosswordGrid, FlatTrie};

/// Dictionary used when no dictionary path is given
const DEFAULT_DICTIONARY_PATH: &str = "resources/dictionaries/french.txt";

#[derive(Parser)]
#[command(about = "Generate French arrow crosswords (mots fléchés)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fill a grid with words from a dictionary
    Solve {
        /// Grid file: comma separated cells, `.` for letters, capelito types for definitions
        grid_path: String,
        /// Dictionary file, one lowercase word per line
        #[arg(short, long, default_value = DEFAULT_DICTIONARY_PATH)]
        dictionary: String,
        /// Seed of the random word order; random when not given
        #[arg(short, long)]
        seed: Option<u64>,
    },
    /// Print the capelitos of a grid without solving it
    Inspect {
        /// Grid file: comma separated cells, `.` for letters, capelito types for definitions
        grid_path: String,
    },
}

fn main() {
    match Cli::parse().command {
        Command::Solve { grid_path, dictionary, seed } => solve(&grid_path, &dictionary, seed),
        Command::Inspect { grid_path } => inspect(&grid_path),
    }
}

fn solve(grid_path: &str, dictionary_path: &str, seed: Option<u64>) {
    // A random seed by default, printed so a grid can be reproduced with `--seed`
    let seed = seed.unwrap_or_else(|| fastrand::u64(..));

    let trie = FlatTrie::from_file_path(dictionary_path).unwrap_or_else(|err| {
        eprintln!("Cannot load dictionary {dictionary_path}: {err}");
        process::exit(1);
    });
    let grid = load_grid(grid_path);
    let mut board = grid.new_board();

    println!("Solving grid with seed {seed}...");
    if !solve_backtrack(&grid, &trie, &mut board, seed) {
        println!("No solution found for this grid configuration.");
        return;
    }

    println!("\nSolution found:\n");
    for row in &board {
        let row_str = String::from_utf8_lossy(row);
        println!("{}", row_str.chars().map(|c| format!("{c} ")).collect::<String>());
    }

    println!("\nCapelitos:");
    for capelito in &grid.capelitos {
        let word = String::from_utf8_lossy(&extract_pattern(&board, capelito)).into_owned();
        println!("  {}: {word}", describe_capelito(capelito));
    }
}

fn inspect(grid_path: &str) {
    let grid = load_grid(grid_path);

    println!("Grid {}x{} with {} capelitos:", grid.height, grid.width, grid.capelitos.len());
    for capelito in &grid.capelitos {
        println!("  {}", describe_capelito(capelito));
    }
}

fn load_grid(grid_path: &str) -> CrosswordGrid {
    CrosswordGrid::from_file_path(grid_path).unwrap_or_else(|err| {
        eprintln!("Cannot load grid {grid_path}: {err}");
        process::exit(1);
    })
}

fn describe_capelito(capelito: &Capelito) -> String {
    format!(
        "{} ({}, {}) -> first letter ({}, {}), length {}",
        capelito.arrowed_place_holder.unicode_char,
        capelito.i,
        capelito.j,
        capelito.first_letter_i,
        capelito.first_letter_j,
        capelito.word_length,
    )
}
