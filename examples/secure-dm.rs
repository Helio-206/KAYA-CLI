use anyhow::Result;
use kaya_sdk::{KayaClient, KayaConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let target = args.next().unwrap_or_else(|| "KY-REPLACE".into());
    let body = args.next().unwrap_or_else(|| "hello secure offline".into());

    let client = KayaClient::new(KayaConfig::default()).await?;
    client.set_callsign("secure-dm-example").await?;
    client.send_secure_direct_message(&target, &body).await?;
    client.stop().await?;
    Ok(())
}
