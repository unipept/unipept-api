use clap::Parser;
use unipept_api::start;

#[derive(Parser, Debug)]
pub struct Arguments {
    #[arg(short, long)]
    index_location: String,
    #[arg(short, long)]
    database_address: String,
    #[arg(short, long)]
    port: u32,
    #[arg(short, long, default_value_t = false)]
    mmap: bool
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    if let Err(e) = start(&args.index_location, &args.database_address, args.port, args.mmap).await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
