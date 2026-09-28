use std::fmt;
use std::fs;
use std::io;
use std::str::FromStr;

use crate::arrowed_place_holder::{get_arrowed_place_holder, ArrowedPlaceHolder};

pub type Board = Vec<Vec<u8>>;

/// Board byte for a letter cell that has not been filled yet
pub const EMPTY_CELL: u8 = b'.';
/// Board byte for a cell holding one or two definitions (capelitos)
pub const DEFINITION_CELL: u8 = b'#';

#[derive(Clone, Debug)]
pub struct Capelito {
    pub id: usize,
    /// Row of the definition cell
    pub i: usize,
    /// Column of the definition cell
    pub j: usize,
    pub arrowed_place_holder: &'static ArrowedPlaceHolder,
    pub first_letter_i: usize,
    pub first_letter_j: usize,
    pub word_length: usize,
}

impl Capelito {
    pub fn is_horizontal(&self) -> bool {
        self.arrowed_place_holder.is_horizontal
    }

    /// Board cell (i, j) of the letter at `position` in this capelito's word
    pub fn letter_cell(&self, position: usize) -> (usize, usize) {
        if self.is_horizontal() {
            (self.first_letter_i, self.first_letter_j + position)
        } else {
            (self.first_letter_i + position, self.first_letter_j)
        }
    }
}

/// A letter cell shared by two capelitos
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Crossing {
    /// Letter index in the word of the capelito owning this crossing
    pub position: usize,
    /// The other capelito using that cell
    pub capelito_id: usize,
}

pub struct CrosswordGrid {
    pub width: usize,
    pub height: usize,
    pub capelitos: Vec<Capelito>,
    /// Crossings of each capelito, indexed by capelito id and ordered by position
    pub crossings: Vec<Vec<Crossing>>,
    /// Starting board: empty and pre-filled letter cells, and definition cells
    pub board: Board,
}

#[derive(Debug)]
pub enum GridError {
    Io(io::Error),
    Empty,
    RaggedRow { i: usize, expected: usize, found: usize },
    InvalidCell { i: usize, j: usize, value: String },
    UnknownCapelitoType { i: usize, j: usize, capelito_type: u8 },
    CapelitoWithoutLetters { i: usize, j: usize, capelito_type: u8 },
}

impl fmt::Display for GridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GridError::Io(err) => write!(f, "cannot read grid file: {err}"),
            GridError::Empty => write!(f, "grid is empty"),
            GridError::RaggedRow { i, expected, found } => {
                write!(f, "row {i} has {found} cells, expected {expected}")
            }
            GridError::InvalidCell { i, j, value } => {
                write!(f, "invalid cell {value:?} at ({i}, {j})")
            }
            GridError::UnknownCapelitoType { i, j, capelito_type } => {
                write!(f, "unknown capelito type {capelito_type} at ({i}, {j})")
            }
            GridError::CapelitoWithoutLetters { i, j, capelito_type } => {
                write!(f, "capelito type {capelito_type} at ({i}, {j}) points to no letter cell")
            }
        }
    }
}

impl std::error::Error for GridError {}

impl From<io::Error> for GridError {
    fn from(err: io::Error) -> Self {
        GridError::Io(err)
    }
}

impl CrosswordGrid {
    pub fn from_file_path(path: &str) -> Result<Self, GridError> {
        fs::read_to_string(path)?.parse()
    }

    pub fn new_board(&self) -> Board {
        self.board.clone()
    }

    fn compute_crossings(&self) -> Vec<Vec<Crossing>> {
        // (capelito id, position) of every capelito using each cell
        let mut cell_capelitos = vec![vec![Vec::new(); self.width]; self.height];
        for capelito in &self.capelitos {
            for position in 0..capelito.word_length {
                let (i, j) = capelito.letter_cell(position);
                cell_capelitos[i][j].push(capelito.id);
            }
        }

        self.capelitos
            .iter()
            .map(|capelito| {
                (0..capelito.word_length)
                    .flat_map(|position| {
                        let (i, j) = capelito.letter_cell(position);
                        cell_capelitos[i][j]
                            .iter()
                            .filter(|&&capelito_id| capelito_id != capelito.id)
                            .map(move |&capelito_id| Crossing { position, capelito_id })
                    })
                    .collect()
            })
            .collect()
    }

    /// Length of the word starting at (i, j), running until a definition cell or the grid edge
    fn word_length_from(&self, mut i: usize, mut j: usize, is_horizontal: bool) -> usize {
        let mut word_length = 0;
        while i < self.height && j < self.width && self.board[i][j] != DEFINITION_CELL {
            word_length += 1;
            if is_horizontal {
                j += 1;
            } else {
                i += 1;
            }
        }
        word_length
    }
}

impl FromStr for CrosswordGrid {
    type Err = GridError;

    /// Parses a comma separated grid: `.` is an empty letter cell, a single letter is a
    /// pre-filled letter cell, and one or two digits are the capelito types of a definition cell.
    fn from_str(content: &str) -> Result<Self, Self::Err> {
        let rows: Vec<Vec<&str>> = content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.split(',').map(str::trim).collect())
            .collect();

        let height = rows.len();
        let width = rows.first().map_or(0, Vec::len);
        if height == 0 || width == 0 {
            return Err(GridError::Empty);
        }

        let mut board = vec![vec![EMPTY_CELL; width]; height];
        // (i, j, capelito_type) in reading order, like the Python implementation
        let mut definitions = Vec::new();

        for (i, row) in rows.iter().enumerate() {
            if row.len() != width {
                return Err(GridError::RaggedRow { i, expected: width, found: row.len() });
            }
            for (j, &value) in row.iter().enumerate() {
                let bytes = value.as_bytes();
                match bytes {
                    [EMPTY_CELL] => {}
                    [letter] if letter.is_ascii_alphabetic() => {
                        board[i][j] = letter.to_ascii_lowercase();
                    }
                    [_] | [_, _] if bytes.iter().all(u8::is_ascii_digit) => {
                        board[i][j] = DEFINITION_CELL;
                        definitions.extend(bytes.iter().map(|digit| (i, j, digit - b'0')));
                    }
                    _ => {
                        return Err(GridError::InvalidCell { i, j, value: value.to_string() });
                    }
                }
            }
        }

        let mut grid = CrosswordGrid { width, height, capelitos: Vec::with_capacity(definitions.len()), crossings: Vec::new(), board };

        for (i, j, capelito_type) in definitions {
            let arrowed_place_holder = get_arrowed_place_holder(capelito_type)
                .ok_or(GridError::UnknownCapelitoType { i, j, capelito_type })?;
            let without_letters = GridError::CapelitoWithoutLetters { i, j, capelito_type };

            let (Some(first_letter_i), Some(first_letter_j)) = (
                i.checked_add_signed(arrowed_place_holder.i_diff),
                j.checked_add_signed(arrowed_place_holder.j_diff),
            ) else {
                return Err(without_letters);
            };

            let word_length =
                grid.word_length_from(first_letter_i, first_letter_j, arrowed_place_holder.is_horizontal);
            if word_length == 0 {
                return Err(without_letters);
            }

            grid.capelitos.push(Capelito {
                id: grid.capelitos.len(),
                i,
                j,
                arrowed_place_holder,
                first_letter_i,
                first_letter_j,
                word_length,
            });
        }

        grid.crossings = grid.compute_crossings();
        Ok(grid)
    }
}

/// Helper to read characters along a capelito's word from the board
pub fn extract_pattern(board: &Board, capelito: &Capelito) -> Vec<u8> {
    (0..capelito.word_length)
        .map(|position| {
            let (i, j) = capelito.letter_cell(position);
            board[i][j]
        })
        .collect()
}

/// Helper to write bytes into the board along a capelito's word
pub fn place_word(board: &mut Board, capelito: &Capelito, word: &[u8]) {
    for (position, &byte) in word.iter().enumerate() {
        let (i, j) = capelito.letter_cell(position);
        board[i][j] = byte;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP_S: &str = "\
36,.,32,.,2
.,.,.,.,.
16,.,.,.,.
.,.,.,.,.
16,.,.,.,2
.,.,.,.,.
16,.,.,.,.
.,.,.,.,.
1,.,.,.,.
";

    fn find_capelito(grid: &CrosswordGrid, i: usize, j: usize, capelito_type: u8) -> &Capelito {
        grid.capelitos
            .iter()
            .find(|c| c.i == i && c.j == j && c.arrowed_place_holder.capelito_type == capelito_type)
            .expect("capelito not found")
    }

    fn assert_capelito(
        grid: &CrosswordGrid,
        (i, j, capelito_type): (usize, usize, u8),
        first_letter: (usize, usize),
        is_horizontal: bool,
        word_length: usize,
    ) {
        let capelito = find_capelito(grid, i, j, capelito_type);
        assert_eq!((capelito.first_letter_i, capelito.first_letter_j), first_letter);
        assert_eq!(capelito.is_horizontal(), is_horizontal);
        assert_eq!(capelito.word_length, word_length);
    }

    #[test]
    fn parses_map_s() {
        let grid: CrosswordGrid = MAP_S.parse().unwrap();

        assert_eq!((grid.height, grid.width), (9, 5));
        assert_eq!(grid.capelitos.len(), 13);
        assert!(grid.capelitos.iter().enumerate().all(|(id, c)| c.id == id));
        assert_eq!(grid.board[0][0], DEFINITION_CELL);
        assert_eq!(grid.board[1][1], EMPTY_CELL);

        assert_capelito(&grid, (0, 0, 3), (0, 1), false, 9);
        assert_capelito(&grid, (0, 0, 6), (1, 0), true, 5);
        assert_capelito(&grid, (0, 2, 3), (0, 3), false, 9);
        assert_capelito(&grid, (0, 2, 2), (1, 2), false, 8);
        assert_capelito(&grid, (0, 4, 2), (1, 4), false, 3);
        assert_capelito(&grid, (4, 0, 1), (4, 1), true, 3);
        assert_capelito(&grid, (4, 4, 2), (5, 4), false, 4);
        assert_capelito(&grid, (8, 0, 1), (8, 1), true, 4);
    }

    #[test]
    fn computes_crossings_with_positions() {
        let grid: CrosswordGrid = MAP_S.parse().unwrap();
        let row_1 = find_capelito(&grid, 0, 0, 6);

        let crossings: Vec<(usize, (usize, usize, u8))> = grid.crossings[row_1.id]
            .iter()
            .map(|crossing| {
                let other = &grid.capelitos[crossing.capelito_id];
                (crossing.position, (other.i, other.j, other.arrowed_place_holder.capelito_type))
            })
            .collect();
        assert_eq!(crossings, vec![(1, (0, 0, 3)), (2, (0, 2, 2)), (3, (0, 2, 3)), (4, (0, 4, 2))]);

        for (id, capelito_crossings) in grid.crossings.iter().enumerate() {
            for crossing in capelito_crossings {
                let cell = grid.capelitos[id].letter_cell(crossing.position);
                let back = grid.crossings[crossing.capelito_id]
                    .iter()
                    .find(|c| c.capelito_id == id)
                    .expect("crossing should be symmetric");
                assert_eq!(grid.capelitos[crossing.capelito_id].letter_cell(back.position), cell);
            }
        }
    }

    #[test]
    fn keeps_pre_filled_letters_in_pattern() {
        let grid: CrosswordGrid = "1,.,A,.".parse().unwrap();
        let capelito = &grid.capelitos[0];

        assert_eq!(extract_pattern(&grid.new_board(), capelito), b".a.");
    }

    #[test]
    fn rejects_ragged_row() {
        let result = "1,.,.\n.,.".parse::<CrosswordGrid>();
        assert!(matches!(result, Err(GridError::RaggedRow { i: 1, expected: 3, found: 2 })));
    }

    #[test]
    fn rejects_unknown_capelito_type() {
        let result = "7,.,.".parse::<CrosswordGrid>();
        assert!(matches!(result, Err(GridError::UnknownCapelitoType { capelito_type: 7, .. })));
    }

    #[test]
    fn rejects_invalid_cell() {
        let result = "1,ab,.".parse::<CrosswordGrid>();
        assert!(matches!(result, Err(GridError::InvalidCell { i: 0, j: 1, .. })));
    }

    #[test]
    fn rejects_capelito_without_letters() {
        let off_grid = ".,.,1".parse::<CrosswordGrid>();
        assert!(matches!(off_grid, Err(GridError::CapelitoWithoutLetters { j: 2, .. })));

        let negative_offset = "5,.,.".parse::<CrosswordGrid>();
        assert!(matches!(negative_offset, Err(GridError::CapelitoWithoutLetters { capelito_type: 5, .. })));

        let onto_definition = "1,2\n.,.".parse::<CrosswordGrid>();
        assert!(matches!(onto_definition, Err(GridError::CapelitoWithoutLetters { capelito_type: 1, .. })));
    }

    #[test]
    fn rejects_empty_grid() {
        assert!(matches!("\n  \n".parse::<CrosswordGrid>(), Err(GridError::Empty)));
    }
}
