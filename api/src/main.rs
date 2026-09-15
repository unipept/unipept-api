use clap::Parser;
use unipept_api::start;

#[derive(Parser, Debug)]
#[command(version)]
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
        tracing::error!(error = &e as &dyn std::error::Error, "failed to start");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Arguments;

    /// Nothing else fails if `#[command(version)]` is dropped: `--version` simply stops existing.
    #[test]
    fn the_binary_reports_its_own_version() {
        assert_eq!(Arguments::command().get_version(), Some(env!("CARGO_PKG_VERSION")));
    }
}
