mod diagnostics;
mod logging;
mod prompt;
mod runtime;

use kaya_events::EventBus;
use kaya_files::{FileStore, FileTransferConfig};
use kaya_persistence::{default_data_dir, ConfigStore, Store};
use kaya_security::{IdentityStore, TrustStore};
use kaya_shared::{KayaError, Result};
use kaya_transport::{MulticastTransport, TransportConfig};
use logging::init_tracing;
use prompt::prompt_callsign;
use runtime::{Runtime, RuntimeInit};
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
    let identity_store = IdentityStore::new(&data_dir);
    let identity_created = !identity_store.path().exists();
    let identity = identity_store.load_or_create(&callsign)?;
    let trust_store = TrustStore::load_or_create(&data_dir)?;
    let file_config = file_transfer_config(&config);
    let file_store = FileStore::new(&data_dir, file_config.download_dir.clone())?;

    let transport = MulticastTransport::bind(transport_config(&config)?)
        .await
        .map_err(|err| KayaError::Transport(err.to_string()))?;
    let bus = EventBus::system_default();

    let mut runtime = Runtime::new(RuntimeInit {
        identity,
        transport,
        store,
        config_store,
        config,
        bus,
        trust_store,
        identity_created,
        file_store,
        file_config,
    });
    runtime.run().await
}

fn file_transfer_config(config: &kaya_persistence::KayaConfig) -> FileTransferConfig {
    FileTransferConfig {
        enabled: config.file_transfer.enabled,
        max_file_size_bytes: config.file_transfer.max_file_size_mb * 1024 * 1024,
        chunk_size: (config.file_transfer.chunk_size_kb * 1024) as usize,
        accept_from_unknown: config.file_transfer.accept_from_unknown,
        download_dir: config.file_transfer.download_dir.clone(),
    }
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
