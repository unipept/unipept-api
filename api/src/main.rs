use clap::Parser;
use unipept_api::start;

#[derive(Parser, Debug)]
pub struct Arguments {
    #[arg(short, long)]
    index_location: String,
    #[arg(short, long)]
    database_address: String,
    #[arg(short, long)]
    port: u32
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    if let Err(e) = start(&args.index_location, &args.database_address, args.port).await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
