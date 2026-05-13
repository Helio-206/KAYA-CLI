mod diagnostics;
mod logging;
mod prompt;
mod runtime;

use kaya_events::EventBus;
use kaya_persistence::{default_data_dir, ConfigStore, Store};
use kaya_shared::{KayaError, NodeId, Result};
use kaya_transport::{MulticastTransport, TransportConfig};
use logging::init_tracing;
use prompt::prompt_callsign;
use runtime::Runtime;
use std::net::Ipv4Addr;

pub async fn run() -> Result<()> {
    let data_dir = default_data_dir();
    let config_store = ConfigStore::new(&data_dir);
    let mut config = config_store.load_or_create()?;
    init_tracing(&config.log_level);

    let store = Store::open(&data_dir)?;
    let callsign = prompt_callsign(config.nickname.as_deref())?;
    config.nickname = Some(callsign.clone());
    config_store.save(&config)?;

    let transport = MulticastTransport::bind(transport_config(&config)?)
        .await
        .map_err(|err| KayaError::Transport(err.to_string()))?;
    let bus = EventBus::system_default();
    let node_id = NodeId::generate().to_string();

    let mut runtime = Runtime::new(
        node_id,
        callsign,
        transport,
        store,
        config_store,
        config,
        bus,
    );
    runtime.run().await
}

fn transport_config(config: &kaya_persistence::KayaConfig) -> Result<TransportConfig> {
    let multicast_ip: Ipv4Addr = config
        .multicast_address
        .parse()
        .map_err(|err| KayaError::Config(format!("invalid multicast address: {err}")))?;

    Ok(TransportConfig {
        multicast_ip,
        port: config.multicast_port,
        loopback: true,
        max_packet_bytes: config.packet_max_bytes,
    })
}
