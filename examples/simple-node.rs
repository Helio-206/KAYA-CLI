use anyhow::Result;
use kaya_sdk::{KayaClient, KayaConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let client = KayaClient::new(KayaConfig::default()).await?;

    client.set_callsign("Helio").await?;
    client.join_room("geral").await?;
    client.send_room_message("geral", "hello offline").await?;

    println!(
        "node={} room={}",
        client.node_id().await,
        client.current_room().await
    );
    client.stop().await?;
    Ok(())
}
