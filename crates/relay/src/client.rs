use crate::errors::{RelayError, RelayResult};
use crate::framing::{read_packet, write_packet};
use crate::policy::RelayPolicy;
use kaya_protocol::Packet;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};

#[derive(Debug, Clone)]
pub struct RelayRegistration {
    pub node_id: String,
    pub callsign: String,
    pub fingerprint: String,
    pub capabilities: Vec<String>,
}

pub struct RelayClient {
    outgoing: mpsc::UnboundedSender<Packet>,
    incoming: mpsc::UnboundedReceiver<Packet>,
    read_task: JoinHandle<RelayResult<()>>,
    write_task: JoinHandle<RelayResult<()>>,
    heartbeat_task: Option<JoinHandle<RelayResult<()>>>,
}

impl RelayClient {
    pub async fn connect(
        url: &str,
        registration: RelayRegistration,
        policy: RelayPolicy,
    ) -> RelayResult<Self> {
        let address = parse_tcp_url(url)?;
        let stream = TcpStream::connect(address).await?;
        let (mut reader, mut writer) = stream.into_split();
        let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<Packet>();
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<Packet>();

        write_packet(
            &mut writer,
            &Packet::relay_register(
                registration.node_id.clone(),
                registration.callsign.clone(),
                registration.fingerprint,
                registration.capabilities,
            ),
        )
        .await?;

        let read_task = tokio::spawn(async move {
            loop {
                let packet = read_packet(&mut reader).await?;
                incoming_tx
                    .send(packet)
                    .map_err(|err| RelayError::ChannelClosed(err.to_string()))?;
            }
        });

        let write_task = tokio::spawn(async move {
            while let Some(packet) = outgoing_rx.recv().await {
                write_packet(&mut writer, &packet).await?;
            }
            Ok(())
        });

        let heartbeat_task = if policy.heartbeat_interval_ms == 0 {
            None
        } else {
            let heartbeat_tx = outgoing_tx.clone();
            let node_id = registration.node_id.clone();
            let callsign = registration.callsign.clone();
            Some(tokio::spawn(async move {
                let mut interval =
                    time::interval(Duration::from_millis(policy.heartbeat_interval_ms));
                loop {
                    interval.tick().await;
                    heartbeat_tx
                        .send(Packet::relay_heartbeat(
                            node_id.clone(),
                            callsign.clone(),
                            "alive",
                        ))
                        .map_err(|err| RelayError::ChannelClosed(err.to_string()))?;
                }
            }))
        };

        Ok(Self {
            outgoing: outgoing_tx,
            incoming: incoming_rx,
            read_task,
            write_task,
            heartbeat_task,
        })
    }

    pub fn send(&self, packet: Packet) -> RelayResult<()> {
        self.outgoing
            .send(packet)
            .map_err(|err| RelayError::ChannelClosed(err.to_string()))
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<Packet> {
        self.outgoing.clone()
    }

    pub async fn recv(&mut self) -> Option<Packet> {
        self.incoming.recv().await
    }
}

impl Drop for RelayClient {
    fn drop(&mut self) {
        self.read_task.abort();
        self.write_task.abort();
        if let Some(heartbeat_task) = &self.heartbeat_task {
            heartbeat_task.abort();
        }
    }
}

fn parse_tcp_url(url: &str) -> RelayResult<&str> {
    url.strip_prefix("tcp://")
        .ok_or_else(|| RelayError::Registration(format!("unsupported relay url: {url}")))
}
