use unipept_api::start;

#[tokio::main]
async fn main() {
    if let Err(e) = start().await {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}
