#[tokio::main]
async fn main() {
    if let Err(err) = kaya_app::run().await {
        eprintln!("kaya: {err}");
        std::process::exit(1);
    }
}
