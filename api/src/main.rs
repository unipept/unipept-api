use clap::Parser;
use unipept_api::start;

#[derive(Parser, Debug)]
pub struct Arguments {
    #[arg(short, long)]
    index_location: String
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();
    if let Err(e) = start(&args.index_location).await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
