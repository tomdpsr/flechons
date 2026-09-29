use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use engine::grid::extract_pattern;
use engine::{solve_backtrack, CrosswordGrid, FlatTrie};
use serde::{Deserialize, Serialize};
use tokio::fs;

pub struct AppState {
    pub trie: FlatTrie,
    pub grids_dir: PathBuf,
}

type SharedState = Arc<AppState>;

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/grids", get(list_grids))
        .route("/api/grids/{name}", get(get_grid))
        .route("/api/solve", post(solve))
        .with_state(state)
}

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    InvalidGrid(String),
    Internal(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            ApiError::BadRequest(error) => (StatusCode::BAD_REQUEST, error),
            ApiError::NotFound(error) => (StatusCode::NOT_FOUND, error),
            ApiError::InvalidGrid(error) => (StatusCode::UNPROCESSABLE_ENTITY, error),
            ApiError::Internal(error) => (StatusCode::INTERNAL_SERVER_ERROR, error),
        };
        (status, Json(ErrorResponse { error })).into_response()
    }
}

#[derive(Deserialize)]
struct SolveRequest {
    /// Grid text, in the same format as the grid files
    grid: String,
    /// A string, as JavaScript numbers cannot hold every `u64`. Random when missing.
    seed: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SolveResponse {
    /// A string, as JavaScript numbers cannot hold every `u64`
    pub seed: String,
    pub solved: bool,
    pub width: usize,
    pub height: usize,
    /// One string per row: `#` for definition cells, `.` for empty letter cells
    pub board: Vec<String>,
    pub capelitos: Vec<CapelitoResponse>,
}

#[derive(Debug, Serialize)]
pub struct CapelitoResponse {
    pub i: usize,
    pub j: usize,
    pub capelito_type: u8,
    pub is_horizontal: bool,
    pub first_letter_i: usize,
    pub first_letter_j: usize,
    pub word_length: usize,
    pub word: String,
}

async fn health() -> &'static str {
    "ok"
}

/// Names of the grid files (`<name>.txt`), sorted
async fn list_grids(State(state): State<SharedState>) -> Result<Json<Vec<String>>, ApiError> {
    let mut entries = fs::read_dir(&state.grids_dir).await.map_err(internal_error)?;
    let mut names = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(internal_error)? {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "txt")
            && let Some(name) = path.file_stem().and_then(|stem| stem.to_str())
            && is_valid_grid_name(name)
        {
            names.push(name.to_owned());
        }
    }
    names.sort();
    Ok(Json(names))
}

async fn get_grid(State(state): State<SharedState>, Path(name): Path<String>) -> Result<String, ApiError> {
    if !is_valid_grid_name(&name) {
        return Err(ApiError::BadRequest(format!("invalid grid name {name:?}")));
    }
    fs::read_to_string(state.grids_dir.join(format!("{name}.txt"))).await.map_err(|err| {
        if err.kind() == io::ErrorKind::NotFound {
            ApiError::NotFound(format!("grid {name:?} not found"))
        } else {
            internal_error(err)
        }
    })
}

async fn solve(
    State(state): State<SharedState>,
    Json(request): Json<SolveRequest>,
) -> Result<Json<SolveResponse>, ApiError> {
    let seed = parse_seed(request.seed.as_deref())?;
    // Solving is CPU-bound and can take seconds: keep it off the async workers
    let response = tokio::task::spawn_blocking(move || solve_grid(&state.trie, &request.grid, seed))
        .await
        .map_err(internal_error)??;
    Ok(Json(response))
}

/// Parses `grid_text` and fills it with words from `trie`
pub fn solve_grid(trie: &FlatTrie, grid_text: &str, seed: u64) -> Result<SolveResponse, ApiError> {
    let grid: CrosswordGrid = grid_text.parse().map_err(|err| ApiError::InvalidGrid(format!("{err}")))?;
    let mut board = grid.new_board();
    let solved = solve_backtrack(&grid, trie, &mut board, seed);

    let capelitos = grid
        .capelitos
        .iter()
        .map(|capelito| CapelitoResponse {
            i: capelito.i,
            j: capelito.j,
            capelito_type: capelito.arrowed_place_holder.capelito_type,
            is_horizontal: capelito.is_horizontal(),
            first_letter_i: capelito.first_letter_i,
            first_letter_j: capelito.first_letter_j,
            word_length: capelito.word_length,
            word: String::from_utf8_lossy(&extract_pattern(&board, capelito)).into_owned(),
        })
        .collect();

    Ok(SolveResponse {
        seed: seed.to_string(),
        solved,
        width: grid.width,
        height: grid.height,
        board: board.iter().map(|row| String::from_utf8_lossy(row).into_owned()).collect(),
        capelitos,
    })
}

fn parse_seed(seed: Option<&str>) -> Result<u64, ApiError> {
    match seed {
        Some(seed) => seed.parse().map_err(|_| ApiError::BadRequest(format!("invalid seed {seed:?}"))),
        None => Ok(fastrand::u64(..)),
    }
}

/// Only lowercase letters, digits and `_`, so a name can never reach outside the grids folder
fn is_valid_grid_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn internal_error(err: impl std::fmt::Display) -> ApiError {
    ApiError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3x3 block of letters, crossed by 3 horizontal and 3 vertical words
    const SQUARE: &str = ".,2,2,2\n1,.,.,.\n1,.,.,.\n1,.,.,.";

    fn sample_trie() -> FlatTrie {
        let mut trie = FlatTrie::new();
        for word in [b"mer", b"tir", b"mat", b"rer", b"sol", b"eau", b"ami"] {
            trie.insert(word);
        }
        trie
    }

    #[test]
    fn solves_grid_with_words_from_the_trie() {
        let trie = sample_trie();
        let response = solve_grid(&trie, SQUARE, 42).unwrap();

        assert!(response.solved);
        assert_eq!(response.seed, "42");
        assert_eq!((response.width, response.height), (4, 4));
        assert_eq!(response.board.len(), 4);
        assert_eq!(response.capelitos.len(), 6);
        for capelito in &response.capelitos {
            assert_eq!(trie.count_matches(capelito.word.as_bytes(), 1), 1, "{} is not a word", capelito.word);
        }
    }

    #[test]
    fn same_seed_gives_same_board() {
        let trie = sample_trie();

        assert_eq!(solve_grid(&trie, SQUARE, 7).unwrap().board, solve_grid(&trie, SQUARE, 7).unwrap().board);
    }

    #[test]
    fn returns_starting_board_without_solution() {
        let mut trie = FlatTrie::new();
        trie.insert(b"mer");
        let response = solve_grid(&trie, SQUARE, 0).unwrap();

        assert!(!response.solved);
        assert_eq!(response.board, [".###", "#...", "#...", "#..."]);
        assert!(response.capelitos.iter().all(|capelito| capelito.word == "..."));
    }

    #[test]
    fn rejects_invalid_grid() {
        let error = solve_grid(&sample_trie(), "7,.,.", 0).unwrap_err();

        assert!(matches!(error, ApiError::InvalidGrid(message) if message.contains("unknown capelito type 7")));
    }

    #[test]
    fn parses_seed() {
        assert_eq!(parse_seed(Some("18204123593817334831")).unwrap(), 18204123593817334831);
        assert!(matches!(parse_seed(Some("abc")), Err(ApiError::BadRequest(_))));
        assert!(parse_seed(None).is_ok());
    }

    #[test]
    fn validates_grid_names() {
        assert!(is_valid_grid_name("grid_s"));
        assert!(is_valid_grid_name("grid_l2"));
        assert!(!is_valid_grid_name(""));
        assert!(!is_valid_grid_name("../secret"));
        assert!(!is_valid_grid_name("A b"));
    }
}
