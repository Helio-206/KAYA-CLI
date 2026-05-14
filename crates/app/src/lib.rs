mod diagnostics;
mod logging;
mod prompt;
mod runtime;

use clap::Parser;
use kaya_events::EventBus;
use kaya_files::{FileStore, FileTransferConfig};
use kaya_mesh::MeshPolicy;
use kaya_persistence::{profile_data_dir, ConfigProfile, ConfigStore, Store};
use kaya_relay::{RelayPolicy, RelayServer};
use kaya_security::{IdentityStore, TrustStore};
use kaya_shared::{KayaError, Result};
use kaya_transport::{MulticastTransport, TransportConfig};
use logging::init_tracing;
use prompt::prompt_callsign_if_needed;
use runtime::{Runtime, RuntimeInit};
use std::net::Ipv4Addr;
use std::path::PathBuf;

pub async fn run() -> Result<()> {
    run_with_args(std::env::args_os()).await
}

pub async fn run_with_args<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let options = RuntimeOptions::parse_from(args);
    if options.version {
        println!("KAYA CLI {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if options.about {
        println!("KAYA CLI {}", about_text());
        return Ok(());
    }

    let data_dir = options.data_dir();
    let config_store = ConfigStore::new(&data_dir);
    let mut config = config_store.load_or_create()?;
    config.apply_profile(options.profile);
    apply_runtime_overrides(&mut config, &options)?;
    init_tracing(&config.log_level);
    config_store.save(&config)?;

    if let Some(RuntimeCommand::Relay(command)) = &options.command {
        return run_relay_server(command, &config).await;
    }

    let store = Store::open(&data_dir)?;
    let callsign = prompt_callsign_if_needed(
        config.nickname.as_deref(),
        options.demo.then_some(options.profile),
    )?;
    config.nickname = Some(callsign.clone());
    let identity_store = IdentityStore::new(&data_dir);
    let identity_created = !identity_store.path().exists();
    let identity = identity_store.load_or_create(&callsign)?;
    let trust_store = TrustStore::load_or_create(&data_dir)?;
    let file_config = file_transfer_config(&config);
    let mesh_policy = mesh_policy(&config);
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
        profile: options.profile,
        demo_mode: options.demo,
        bus,
        trust_store,
        identity_created,
        file_store,
        file_config,
        mesh_policy,
    });
    runtime.run().await
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "kaya",
    disable_version_flag = true,
    disable_help_subcommand = true
)]
pub struct RuntimeOptions {
    #[arg(long)]
    pub demo: bool,
    #[arg(long, value_name = "PROFILE", default_value = "default")]
    pub profile: ConfigProfile,
    #[arg(long = "data-dir", value_name = "PATH")]
    pub data_dir_override: Option<PathBuf>,
    #[arg(long = "relay", value_name = "URL")]
    pub relay_url: Option<String>,
    #[arg(long)]
    pub local: bool,
    #[arg(long)]
    pub version: bool,
    #[arg(long)]
    pub about: bool,
    #[command(subcommand)]
    pub command: Option<RuntimeCommand>,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum RuntimeCommand {
    Relay(RelayServerCommand),
}

#[derive(Debug, Clone, clap::Args)]
pub struct RelayServerCommand {
    #[arg(long, value_name = "ADDR", default_value = "0.0.0.0:7777")]
    pub bind: String,
}

impl RuntimeOptions {
    pub fn data_dir(&self) -> PathBuf {
        if let Some(path) = &self.data_dir_override {
            return path.clone();
        }

        if self.demo {
            return profile_data_dir(ConfigProfile::Demo);
        }

        profile_data_dir(self.profile)
    }
}

pub(crate) fn about_text() -> &'static str {
    "0.1.0\nLocal-first communication for temporary digital communities.\nUse --demo for isolated presentation mode, --relay tcp://host:7777 for WAN bridging, or kaya relay --bind 0.0.0.0:7777 to host a relay."
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

fn mesh_policy(config: &kaya_persistence::KayaConfig) -> MeshPolicy {
    MeshPolicy {
        enabled: config.mesh.enabled,
        max_ttl: config.mesh.max_ttl,
        allow_relay_for_unknown: config.mesh.allow_relay_for_unknown,
        allow_relay_for_blocked: config.mesh.allow_relay_for_blocked,
        relay_encrypted_only: config.mesh.relay_encrypted_only,
        route_expiry_seconds: config.mesh.route_expiry_seconds,
        max_seen_packets: config.mesh.max_seen_packets,
    }
}

fn relay_policy(config: &kaya_persistence::KayaConfig) -> RelayPolicy {
    RelayPolicy {
        allow_unknown: config.file_transfer.accept_from_unknown,
        max_clients: 100,
        heartbeat_interval_ms: config.relay.heartbeat_interval_ms,
        connection_timeout_ms: config.relay.connection_timeout_ms,
        rooms: kaya_relay::RelayRoomPolicy {
            enabled: config.relay.rooms.enabled,
            broadcast: config.relay.rooms.broadcast,
        },
        file_transfer: kaya_relay::RelayFileTransferPolicy {
            enabled: config.relay.file_transfer.enabled,
            allow_chunks: config.relay.file_transfer.allow_chunks,
            max_file_size_mb: config.relay.file_transfer.max_file_size_mb,
        },
    }
}

fn apply_runtime_overrides(
    config: &mut kaya_persistence::KayaConfig,
    options: &RuntimeOptions,
) -> Result<()> {
    if let Some(url) = &options.relay_url {
        if !url.starts_with("tcp://") {
            return Err(KayaError::Config("relay url must start with tcp://".into()));
        }
        config.relay.enabled = true;
        config.relay.url = Some(url.clone());
    }
    if options.local {
        config.relay.rooms.bridge_local = true;
        config.relay.prefer_local = true;
    }
    Ok(())
}

async fn run_relay_server(
    command: &RelayServerCommand,
    config: &kaya_persistence::KayaConfig,
) -> Result<()> {
    let server = RelayServer::bind(&command.bind, relay_policy(config))
        .await
        .map_err(|err| KayaError::Transport(err.to_string()))?;
    let bind_addr = server.bind_addr().to_string();
    let handle = server.spawn();
    println!("KAYA relay listening on tcp://{bind_addr}");
    tokio::signal::ctrl_c()
        .await
        .map_err(|err| KayaError::Transport(err.to_string()))?;
    handle
        .shutdown()
        .await
        .map_err(|err| KayaError::Transport(err.to_string()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_demo_and_profile_options() {
        let options = RuntimeOptions::parse_from(["kaya", "--demo", "--profile", "paranoid"]);

        assert!(options.demo);
        assert_eq!(options.profile, ConfigProfile::Paranoid);
        assert!(options.data_dir().ends_with(".kaya-demo"));
    }

    #[test]
    fn parses_relay_options() {
        let options =
            RuntimeOptions::parse_from(["kaya", "--relay", "tcp://relay.example:7777", "--local"]);

        assert_eq!(
            options.relay_url.as_deref(),
            Some("tcp://relay.example:7777")
        );
        assert!(options.local);
    }

    #[test]
    fn parses_relay_subcommand() {
        let options = RuntimeOptions::parse_from(["kaya", "relay", "--bind", "127.0.0.1:9000"]);

        match options.command {
            Some(RuntimeCommand::Relay(command)) => {
                assert_eq!(command.bind, "127.0.0.1:9000");
            }
            None => panic!("relay subcommand missing"),
        }
    }

    #[test]
    fn uses_profile_directory_when_not_demo() {
        let options = RuntimeOptions::parse_from(["kaya", "--profile", "lab"]);

        assert_eq!(options.profile, ConfigProfile::Lab);
        assert!(options.data_dir().ends_with(".kaya-lab"));
    }
}
