mod api;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;

use clap::Parser;
use engine::dictionary::DEFAULT_DICTIONARY_PATH;
use engine::FlatTrie;
use tokio::net::TcpListener;

use api::AppState;

#[derive(Parser)]
#[command(about = "HTTP API to generate French arrow crosswords (mots fléchés)")]
struct Cli {
    /// Dictionary file, one lowercase word per line
    #[arg(short, long, default_value = DEFAULT_DICTIONARY_PATH)]
    dictionary: String,
    /// Folder holding the grid files, named `<name>.txt`
    #[arg(short, long, default_value = "resources/grids")]
    grids_dir: PathBuf,
    /// Address to listen on
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    address: SocketAddr,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Loaded once, then shared by every request
    let trie = FlatTrie::from_file_path(&cli.dictionary).unwrap_or_else(|err| {
        eprintln!("Cannot load dictionary {}: {err}", cli.dictionary);
        process::exit(1);
    });
    let state = Arc::new(AppState { trie, grids_dir: cli.grids_dir });

    let listener = TcpListener::bind(cli.address).await.unwrap_or_else(|err| {
        eprintln!("Cannot listen on {}: {err}", cli.address);
        process::exit(1);
    });
    println!("Listening on http://{}", cli.address);

    if let Err(err) = axum::serve(listener, api::router(state)).await {
        eprintln!("Server error: {err}");
        process::exit(1);
    }
}
