use clap::Parser;
use unipept_api::start;

#[derive(Parser, Debug)]
pub struct Arguments {
    #[arg(short, long)]
    index_location: String,
    #[arg(short, long)]
    datastore_location: String,
    #[arg(short('a'), long)]
    database_address: String
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    if let Err(e) = start(&args.index_location, &args.datastore_location, &args.database_address).await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
