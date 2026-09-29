use crate::grid::{extract_pattern, place_word, EMPTY_CELL};
use crate::{Board, CrosswordGrid, FlatTrie};

/// Candidate counts above this are only used to order capelitos, so they are capped
const CANDIDATE_COUNT_LIMIT: usize = 256;

/// Fills every capelito of the grid with words from the trie, trying candidates in a
/// random order driven by `seed`: the same seed always gives the same board.
/// Returns `false` and leaves the board unchanged when there is no solution.
pub fn solve_backtrack(grid: &CrosswordGrid, trie: &FlatTrie, board: &mut Board, seed: u64) -> bool {
    Solver::new(grid, trie, board, seed).is_some_and(|mut solver| solver.backtrack())
}

struct Solver<'a> {
    grid: &'a CrosswordGrid,
    trie: &'a FlatTrie,
    board: &'a mut Board,
    is_filled: Vec<bool>,
    /// Number of words matching each capelito's pattern, capped at `CANDIDATE_COUNT_LIMIT`
    candidate_counts: Vec<usize>,
    /// (capelito id, previous count), undone back to a saved length on backtrack
    trail: Vec<(usize, usize)>,
    rng: fastrand::Rng,
}

impl<'a> Solver<'a> {
    /// Returns `None` when a capelito has no candidate on the starting board
    fn new(grid: &'a CrosswordGrid, trie: &'a FlatTrie, board: &'a mut Board, seed: u64) -> Option<Self> {
        let candidate_counts: Vec<usize> = grid
            .capelitos
            .iter()
            .map(|capelito| trie.count_matches(&extract_pattern(board, capelito), CANDIDATE_COUNT_LIMIT))
            .collect();
        if candidate_counts.contains(&0) {
            return None;
        }

        Some(Self {
            grid,
            trie,
            board,
            is_filled: vec![false; grid.capelitos.len()],
            candidate_counts,
            trail: Vec::new(),
            rng: fastrand::Rng::with_seed(seed),
        })
    }

    fn backtrack(&mut self) -> bool {
        let Some(capelito_id) = self.next_capelito() else {
            return true; // Every capelito is filled
        };
        let capelito = &self.grid.capelitos[capelito_id];

        let pattern = extract_pattern(self.board, capelito);
        let mut candidates = self.trie.find_matches(&pattern);
        self.is_filled[capelito_id] = true;

        // Lazy Fisher-Yates: move a random untried candidate to the end of the untried part,
        // so only the candidates actually tried get shuffled
        for remaining in (1..=candidates.len()).rev() {
            candidates.swap(self.rng.usize(..remaining), remaining - 1);
            let word = &candidates[remaining - 1];

            let trail_length = self.trail.len();
            place_word(self.board, capelito, word);

            if self.forward_check(capelito_id, &pattern) && self.backtrack() {
                return true;
            }

            self.undo_trail(trail_length);
        }

        // No candidate works with the current board: restore the pattern
        place_word(self.board, capelito, &pattern);
        self.is_filled[capelito_id] = false;
        false
    }

    /// Unfilled capelito with the fewest candidates, then the longest word, then the lowest id
    fn next_capelito(&self) -> Option<usize> {
        (0..self.grid.capelitos.len())
            .filter(|&capelito_id| !self.is_filled[capelito_id])
            .min_by_key(|&capelito_id| {
                (self.candidate_counts[capelito_id], usize::MAX - self.grid.capelitos[capelito_id].word_length)
            })
    }

    /// Recounts the unfilled crossings whose shared cell was empty in `pattern_before`.
    /// Returns `false` as soon as one of them has no candidate left.
    fn forward_check(&mut self, capelito_id: usize, pattern_before: &[u8]) -> bool {
        for crossing in &self.grid.crossings[capelito_id] {
            if self.is_filled[crossing.capelito_id] || pattern_before[crossing.position] != EMPTY_CELL {
                continue;
            }
            let crossing_capelito = &self.grid.capelitos[crossing.capelito_id];
            let count = self
                .trie
                .count_matches(&extract_pattern(self.board, crossing_capelito), CANDIDATE_COUNT_LIMIT);

            self.trail.push((crossing.capelito_id, self.candidate_counts[crossing.capelito_id]));
            self.candidate_counts[crossing.capelito_id] = count;
            if count == 0 {
                return false;
            }
        }
        true
    }

    /// Replace the current candidates count with the previous iteration when backtracking
    fn undo_trail(&mut self, trail_length: usize) {
        while self.trail.len() > trail_length {
            let (capelito_id, previous_count) = self.trail.pop().unwrap();
            self.candidate_counts[capelito_id] = previous_count;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// A 3x3 block of letters, crossed by 3 horizontal and 3 vertical words
    const SQUARE: &str = ".,2,2,2\n1,.,.,.\n1,.,.,.\n1,.,.,.";

    fn sample_trie() -> FlatTrie {
        let mut trie = FlatTrie::new();
        for word in [b"mer", b"tir", b"mat", b"rer", b"sol", b"eau", b"ami"] {
            trie.insert(word);
        }
        trie
    }

    fn assert_every_capelito_is_a_word(grid: &CrosswordGrid, trie: &FlatTrie, board: &Board) {
        for capelito in &grid.capelitos {
            let pattern = extract_pattern(board, capelito);
            assert_eq!(trie.count_matches(&pattern, 1), 1, "{:?} is not a word", String::from_utf8_lossy(&pattern));
        }
    }

    #[test]
    fn solves_square_grid() {
        let grid: CrosswordGrid = SQUARE.parse().unwrap();
        let trie = sample_trie();
        let mut board = grid.new_board();

        assert!(solve_backtrack(&grid, &trie, &mut board, 0));
        assert_every_capelito_is_a_word(&grid, &trie, &board);
    }

    #[test]
    fn same_seed_gives_same_board() {
        let grid: CrosswordGrid = SQUARE.parse().unwrap();
        let trie = sample_trie();
        let mut first_board = grid.new_board();
        let mut second_board = grid.new_board();

        assert!(solve_backtrack(&grid, &trie, &mut first_board, 42));
        assert!(solve_backtrack(&grid, &trie, &mut second_board, 42));
        assert_eq!(first_board, second_board);
    }

    #[test]
    fn different_seeds_pick_different_words() {
        let grid: CrosswordGrid = "1,.,.,.".parse().unwrap();
        let trie = sample_trie();

        let words: HashSet<Vec<u8>> = (0..20)
            .map(|seed| {
                let mut board = grid.new_board();
                assert!(solve_backtrack(&grid, &trie, &mut board, seed));
                extract_pattern(&board, &grid.capelitos[0])
            })
            .collect();
        assert!(words.len() > 1, "20 seeds all picked {words:?}");
    }

    #[test]
    fn keeps_pre_filled_letters() {
        let grid: CrosswordGrid = ".,2,2,2\n1,.,.,.\n1,.,.,.\n1,.,.,r".parse().unwrap();
        let trie = sample_trie();
        let mut board = grid.new_board();

        assert!(solve_backtrack(&grid, &trie, &mut board, 0));
        assert_every_capelito_is_a_word(&grid, &trie, &board);
        assert_eq!(board[3][3], b'r');
    }

    #[test]
    fn leaves_board_unchanged_without_solution() {
        let grid: CrosswordGrid = SQUARE.parse().unwrap();
        let mut trie = FlatTrie::new();
        trie.insert(b"mer");
        trie.insert(b"tir");
        let mut board = grid.new_board();

        assert!(!solve_backtrack(&grid, &trie, &mut board, 0));
        assert_eq!(board, grid.new_board());
    }

    #[test]
    fn fails_without_candidate_on_starting_board() {
        let grid: CrosswordGrid = ".,2,2,2\n1,x,.,.\n1,.,.,.\n1,.,.,.".parse().unwrap();
        let trie = sample_trie();
        let mut board = grid.new_board();

        assert!(!solve_backtrack(&grid, &trie, &mut board, 0));
        assert_eq!(board, grid.new_board());
    }

    #[test]
    fn forward_check_rejects_word_leaving_a_crossing_without_candidate() {
        let grid: CrosswordGrid = SQUARE.parse().unwrap();
        let trie = sample_trie();
        let mut board = grid.new_board();
        let mut solver = Solver::new(&grid, &trie, &mut board, 0).unwrap();

        // First row "eau": no sample word starts with "u", so the last column is dead
        let first_row = grid.capelitos.iter().find(|c| c.is_horizontal() && c.first_letter_i == 1).unwrap();
        let counts_before = solver.candidate_counts.clone();
        solver.is_filled[first_row.id] = true;
        place_word(solver.board, first_row, b"eau");

        assert!(!solver.forward_check(first_row.id, b"..."));
        solver.undo_trail(0);
        assert_eq!(solver.candidate_counts, counts_before);
    }
}
