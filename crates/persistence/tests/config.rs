use kaya_persistence::{ConfigStore, KayaConfig};
use uuid::Uuid;

#[test]
fn config_validation_rejects_bad_timeout_relationship() {
    let config = KayaConfig {
        heartbeat_interval_secs: 10,
        peer_timeout_secs: 10,
        ..KayaConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn missing_config_file_is_created_with_defaults() {
    let path = std::env::temp_dir().join(format!("kaya-config-create-{}", Uuid::new_v4()));
    let store = ConfigStore::new(&path);

    let config = store.load_or_create().unwrap();

    assert_eq!(config.default_room, "geral");
    assert!(store.path().exists());

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn mesh_config_validation_rejects_zero_limits() {
    let mut config = KayaConfig::default();
    config.mesh.max_ttl = 0;

    assert!(config.validate().is_err());

    let mut config = KayaConfig::default();
    config.mesh.route_expiry_seconds = 0;

    assert!(config.validate().is_err());
}
