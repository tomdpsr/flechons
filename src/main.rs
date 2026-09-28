use std::env;
use std::process;

use flechons::grid::extract_pattern;
use flechons::{solve_backtrack, Capelito, CrosswordGrid, FlatTrie};

/// Dictionary used when no dictionary path is given
const DEFAULT_DICTIONARY_PATH: &str = "resources/dictionaries/french.txt";

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

fn exit_with_usage() -> ! {
    eprintln!("Usage: flechons <grid_path> [dictionary_path (default: {DEFAULT_DICTIONARY_PATH})] [--seed <n>]");
    process::exit(1);
}

fn main() {
    let mut positional_args = Vec::new();
    let mut seed = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--seed" {
            let value = args.next().and_then(|value| value.parse::<u64>().ok());
            seed = Some(value.unwrap_or_else(|| exit_with_usage()));
        } else {
            positional_args.push(arg);
        }
    }
    let mut positional_args = positional_args.into_iter();
    let grid_path = positional_args.next().unwrap_or_else(|| exit_with_usage());
    let dictionary_path = positional_args.next().unwrap_or_else(|| DEFAULT_DICTIONARY_PATH.to_string());
    if positional_args.next().is_some() {
        exit_with_usage();
    }
    // A random seed by default, printed so a grid can be reproduced with `--seed`
    let seed = seed.unwrap_or_else(|| fastrand::u64(..));

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
        println!("  {}", describe_capelito(capelito));
    }

    // 3. Initialize the board from the grid skeleton
    let mut board = grid.new_board();

    // 4. Solve grid
    println!("Solving grid with seed {seed}...");
    if solve_backtrack(&grid, &trie, &mut board, seed) {
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
    } else {
        println!("No solution found for this grid configuration.");
    }
}
