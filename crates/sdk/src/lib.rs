use std::path::PathBuf;
use std::sync::Arc;

pub use kaya_core::{
    CoreConfig as KayaConfig, KayaCommand, KayaCommandRegistry, KayaCore, KayaMeshDiagnostics,
    KayaRuntimeEvent, KayaTransport, MockTransport, MockTransportHandle,
};
pub use kaya_events::KayaEvent;
pub use kaya_files::{TransferSecurity, TransferSession, TransferStatus};
pub use kaya_mesh::RouteEntry;
pub use kaya_peer::PeerSnapshot;
pub use kaya_rooms::RoomSummary;
pub use kaya_security::{SecureSessionView, TrustStatus, TrustedPeer};
pub use kaya_shared::PresenceStatus;

#[derive(Clone)]
pub struct KayaClient {
    core: KayaCore,
}

impl KayaClient {
    pub async fn start(config: KayaConfig) -> kaya_shared::Result<Self> {
        Self::new(config).await
    }

    pub async fn new(config: KayaConfig) -> kaya_shared::Result<Self> {
        let core = KayaCore::new(config).await?;
        Ok(Self { core })
    }

    pub async fn with_transport(
        config: KayaConfig,
        transport: Arc<dyn KayaTransport>,
    ) -> kaya_shared::Result<Self> {
        let core = KayaCore::with_transport(config, transport).await?;
        Ok(Self { core })
    }

    pub fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<KayaEvent> {
        self.core.subscribe()
    }

    pub fn help_text(&self) -> String {
        self.core.help_text()
    }

    pub async fn execute_input(&self, input: &str) -> kaya_shared::Result<bool> {
        self.core.execute_input(input).await
    }

    pub async fn set_callsign(&self, callsign: &str) -> kaya_shared::Result<()> {
        self.core.set_callsign(callsign).await
    }

    pub async fn set_presence(&self, presence: PresenceStatus) -> kaya_shared::Result<()> {
        self.core.set_presence(presence).await
    }

    pub async fn join_room(&self, room: &str) -> kaya_shared::Result<()> {
        self.core.join_room(room).await
    }

    pub async fn send_room_message(&self, room: &str, body: &str) -> kaya_shared::Result<()> {
        self.core.send_room_message(room, body).await
    }

    pub async fn send_direct_message(&self, target: &str, body: &str) -> kaya_shared::Result<()> {
        self.core.send_direct_message(target, body).await
    }

    pub async fn send_secure_direct_message(
        &self,
        target: &str,
        body: &str,
    ) -> kaya_shared::Result<()> {
        self.core.send_secure_direct_message(target, body).await
    }

    pub async fn send_file(
        &self,
        target: &str,
        path: impl Into<PathBuf>,
    ) -> kaya_shared::Result<String> {
        self.core.send_file(target, path).await
    }

    pub async fn request_route(&self, destination_node: &str) -> kaya_shared::Result<()> {
        self.core.request_route(destination_node).await
    }

    pub async fn trust_peer(&self, target: &str) -> kaya_shared::Result<()> {
        self.core.trust_peer(target).await
    }

    pub async fn block_peer(&self, target: &str) -> kaya_shared::Result<()> {
        self.core.block_peer(target).await
    }

    pub async fn untrust_peer(&self, target: &str) -> kaya_shared::Result<()> {
        self.core.untrust_peer(target).await
    }

    pub async fn node_id(&self) -> String {
        self.core.node_id().await
    }

    pub async fn callsign(&self) -> String {
        self.core.callsign().await
    }

    pub async fn current_room(&self) -> String {
        self.core.current_room().await
    }

    pub async fn list_peers(&self) -> Vec<PeerSnapshot> {
        self.core.list_peers().await
    }

    pub async fn list_rooms(&self) -> Vec<RoomSummary> {
        self.core.list_rooms().await
    }

    pub async fn inspect_routes(&self) -> Vec<RouteEntry> {
        self.core.list_routes().await
    }

    pub async fn mesh_status(&self) -> KayaMeshDiagnostics {
        self.core.mesh_diagnostics().await
    }

    pub async fn trust_list(&self) -> Vec<TrustedPeer> {
        self.core.trust_list().await
    }

    pub async fn secure_sessions(&self) -> Vec<SecureSessionView> {
        self.core.secure_sessions().await
    }

    pub async fn file_transfers(&self) -> Vec<TransferSession> {
        self.core.file_transfers().await
    }

    pub async fn stop(&self) -> kaya_shared::Result<()> {
        self.core.stop().await
    }
}
