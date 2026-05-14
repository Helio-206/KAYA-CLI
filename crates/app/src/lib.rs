mod diagnostics;
mod logging;
mod prompt;
mod runtime;

use clap::Parser;
use kaya_events::EventBus;
use kaya_files::{FileStore, FileTransferConfig};
use kaya_mesh::MeshPolicy;
use kaya_persistence::{profile_data_dir, ConfigProfile, ConfigStore, Store};
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
    init_tracing(&config.log_level);

    let store = Store::open(&data_dir)?;
    let callsign = prompt_callsign_if_needed(
        config.nickname.as_deref(),
        options.demo.then_some(options.profile),
    )?;
    config.nickname = Some(callsign.clone());
    config_store.save(&config)?;
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
    #[arg(long)]
    pub version: bool,
    #[arg(long)]
    pub about: bool,
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
    "0.1.0\nLocal-first communication for temporary digital communities.\nUse --demo for isolated presentation mode or --profile <default|demo|lab|paranoid>."
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
    fn uses_profile_directory_when_not_demo() {
        let options = RuntimeOptions::parse_from(["kaya", "--profile", "lab"]);

        assert_eq!(options.profile, ConfigProfile::Lab);
        assert!(options.data_dir().ends_with(".kaya-lab"));
    }
}
