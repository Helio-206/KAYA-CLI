use anyhow::Result;
use kaya_sdk::{KayaClient, KayaConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let route_target = std::env::args().nth(1);
    let client = KayaClient::new(KayaConfig::default()).await?;
    client.set_callsign("mesh-status").await?;

    if let Some(target) = route_target {
        client.request_route(&target).await?;
    }

    let routes = client.inspect_routes().await;
    let diagnostics = client.mesh_status().await;
    println!(
        "routes={} relayed={} delivered={} dropped={}",
        routes.len(),
        diagnostics.relayed_packets,
        diagnostics.delivered_packets,
        diagnostics.dropped_packets
    );
    for route in routes {
        println!(
            "{} via {} hops={} score={}",
            route.destination_node, route.next_hop, route.hop_count, route.score
        );
    }

    client.stop().await?;
    Ok(())
}
