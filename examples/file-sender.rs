use anyhow::Result;
use kaya_sdk::{KayaClient, KayaConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let target = args.next().unwrap_or_else(|| "KY-REPLACE".into());
    let path = args.next().unwrap_or_else(|| "./README.md".into());

    let client = KayaClient::new(KayaConfig::default()).await?;
    client.set_callsign("file-sender").await?;
    let file_id = client.send_file(&target, path).await?;
    println!("offered file with id {file_id}");
    client.stop().await?;
    Ok(())
}
