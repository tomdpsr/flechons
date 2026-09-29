# flechons

Generator of French arrow crosswords (mots fléchés), using a backtracking solver on a trie dictionary.

## Layout

```
Cargo.toml          workspace root
crates/
  engine/           library: grid parsing, dictionary trie, solver
  cli/              `flechons` command line (solve / inspect)
  server/           HTTP API (axum)
frontend/           web UI (not scaffolded yet)
resources/
  dictionaries/     one lowercase word per line
  grids/            grid skeletons: `.` for letters, capelito types (1-6) for definitions
```

## Run

From the repository root (default paths are relative to it):

```bash
# Command line
cargo run --release -p cli -- solve resources/grids/grid_s.txt [--seed 42] [--dictionary path]
cargo run -p cli -- inspect resources/grids/grid_l.txt

# Server, on http://127.0.0.1:3000
cargo run --release -p server [-- --address 0.0.0.0:3000 --dictionary path --grids-dir path]

curl localhost:3000/api/grids
curl localhost:3000/api/grids/grid_s
curl -X POST localhost:3000/api/solve -H 'content-type: application/json' \
  -d '{"grid": ".,2,2,2\n1,.,.,.\n1,.,.,.\n1,.,.,.", "seed": "42"}'

# Tests and lints
cargo test --workspace
cargo clippy --workspace --all-targets
```

The seed is a string in the API, as JavaScript numbers cannot hold every `u64`.
