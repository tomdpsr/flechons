use std::env;
use std::process;

use flechons::{solve_backtrack, CrosswordGrid, FlatTrie};

/// Dictionary used when no dictionary path is given
const DEFAULT_DICTIONARY_PATH: &str = "resources/dictionaries/french.txt";

fn main() {
    let mut args = env::args().skip(1);
    let Some(grid_path) = args.next() else {
        eprintln!("Usage: flechons <grid_path> [dictionary_path (default: {DEFAULT_DICTIONARY_PATH})]");
        process::exit(1);
    };
    let dictionary_path = args.next().unwrap_or_else(|| DEFAULT_DICTIONARY_PATH.to_string());

    // 1. Load the dictionary, one lowercase word per line
    let trie = FlatTrie::from_file_path(&dictionary_path).unwrap_or_else(|err| {
        eprintln!("Cannot load dictionary {dictionary_path}: {err}");
        process::exit(1);
    });

    // 2. Load the grid skeleton from its text file
    let grid = CrosswordGrid::from_file_path(&grid_path).unwrap_or_else(|err| {
        eprintln!("Cannot load grid {grid_path}: {err}");
        process::exit(1);
    });

    println!("Grid {}x{} with {} capelitos:", grid.height, grid.width, grid.capelitos.len());
    for capelito in &grid.capelitos {
        println!(
            "  {} ({}, {}) -> first letter ({}, {}), length {}",
            capelito.arrowed_place_holder.unicode_char,
            capelito.i,
            capelito.j,
            capelito.first_letter_i,
            capelito.first_letter_j,
            capelito.word_length,
        );
    }

    // 3. Initialize the board from the grid skeleton
    let mut board = grid.new_board();

    // 4. Solve grid
    println!("Solving grid...");
    if solve_backtrack(&grid, &trie, &mut board) {
        println!("\nSolution found:\n");
        for row in &board {
            let row_str = String::from_utf8_lossy(row);
            println!("{}", row_str.chars().map(|c| format!("{c} ")).collect::<String>());
        }
    } else {
        println!("No solution found for this grid configuration.");
    }
}
