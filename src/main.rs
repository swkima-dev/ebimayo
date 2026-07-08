use ebimayo::agent;

#[tokio::main]
async fn main() {
    if let Err(e) = agent::run().await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
