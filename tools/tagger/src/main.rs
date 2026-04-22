use anyhow::Result;
use clap::Parser;

mod embed;
mod extract;
mod index;

#[derive(Parser, Debug)]
#[command(about = "Upsert note embedding and return top-10 similar notes")]
struct Args {
    /// Path to the markdown note (relative to vault root)
    note: String,

    /// Number of similar notes to return
    #[arg(short, long, default_value_t = 10)]
    top_k: usize,

    /// Path to the vector index directory
    #[arg(long, default_value = ".tagger/index")]
    index_dir: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let (title, body) = extract::read_note(&args.note)?;
    let embedding = embed::embed_text(&body).await?;

    let results = index::upsert_and_search(
        &args.index_dir,
        &args.note,
        &title,
        &embedding,
        args.top_k,
    )?;

    for title in results {
        println!("{}", title);
    }

    Ok(())
}
