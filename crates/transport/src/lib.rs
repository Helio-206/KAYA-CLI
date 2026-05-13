use kaya_protocol::{decode_with_limit, encode, Packet, ProtocolError};
use kaya_shared::{MAX_PACKET_BYTES, MULTICAST_PORT};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::collections::{HashSet, VecDeque};
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use thiserror::Error;
use tokio::net::UdpSocket;
use tracing::{debug, trace};
use uuid::Uuid;

pub type TransportResult<T> = std::result::Result<T, TransportError>;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
}

#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub multicast_ip: Ipv4Addr,
    pub port: u16,
    pub loopback: bool,
    pub max_packet_bytes: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            multicast_ip: Ipv4Addr::new(239, 71, 0, 1),
            port: MULTICAST_PORT,
            loopback: true,
            max_packet_bytes: MAX_PACKET_BYTES,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MulticastTransport {
    socket: Arc<UdpSocket>,
    multicast_addr: SocketAddr,
    max_packet_bytes: usize,
}

impl MulticastTransport {
    pub async fn bind(config: TransportConfig) -> TransportResult<Self> {
        let socket = create_multicast_socket(&config)?;
        let socket = UdpSocket::from_std(socket)?;
        let multicast_addr = SocketAddr::V4(SocketAddrV4::new(config.multicast_ip, config.port));
        debug!(%multicast_addr, "kaya multicast transport bound");
        Ok(Self {
            socket: Arc::new(socket),
            multicast_addr,
            max_packet_bytes: config.max_packet_bytes,
        })
    }

    pub async fn bind_default() -> TransportResult<Self> {
        Self::bind(TransportConfig::default()).await
    }

    pub async fn send_packet(&self, packet: &Packet) -> TransportResult<usize> {
        let bytes = encode(packet)?;
        let sent = self.socket.send_to(&bytes, self.multicast_addr).await?;
        trace!(packet_id = %packet.packet_id, packet_type = ?packet.packet_type, bytes = sent, "packet sent");
        Ok(sent)
    }

    pub async fn recv_packet(&self) -> TransportResult<(Packet, SocketAddr, usize)> {
        let mut buffer = vec![0_u8; self.max_packet_bytes];
        let (len, addr) = self.socket.recv_from(&mut buffer).await?;
        let packet = decode_datagram_with_limit(&buffer[..len], self.max_packet_bytes)?;
        trace!(packet_id = %packet.packet_id, packet_type = ?packet.packet_type, %addr, "packet received");
        Ok((packet, addr, len))
    }

    pub fn multicast_addr(&self) -> SocketAddr {
        self.multicast_addr
    }
}

fn create_multicast_socket(config: &TransportConfig) -> TransportResult<std::net::UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;

    let bind_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, config.port));
    socket.bind(&SockAddr::from(bind_addr))?;
    socket.join_multicast_v4(&config.multicast_ip, &Ipv4Addr::UNSPECIFIED)?;
    socket.set_multicast_loop_v4(config.loopback)?;
    socket.set_multicast_ttl_v4(1)?;
    socket.set_nonblocking(true)?;

    Ok(socket.into())
}

pub fn decode_datagram(bytes: &[u8]) -> TransportResult<Packet> {
    decode_datagram_with_limit(bytes, MAX_PACKET_BYTES)
}

pub fn decode_datagram_with_limit(bytes: &[u8], max_bytes: usize) -> TransportResult<Packet> {
    decode_with_limit(bytes, max_bytes).map_err(Into::into)
}

#[derive(Debug, Clone)]
pub struct PacketDeduplicator {
    capacity: usize,
    seen: HashSet<Uuid>,
    order: VecDeque<Uuid>,
}

impl PacketDeduplicator {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            seen: HashSet::new(),
            order: VecDeque::new(),
        }
    }

    pub fn observe(&mut self, packet_id: Uuid) -> bool {
        if self.seen.contains(&packet_id) {
            return false;
        }

        self.seen.insert(packet_id);
        self.order.push_back(packet_id);
        while self.order.len() > self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.seen.remove(&oldest);
            }
        }
        true
    }

    pub fn len(&self) -> usize {
        self.seen.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_valid_datagram() {
        let packet = Packet::hello("KY-71AF92", "Ana", "geral");
        let bytes = encode(&packet).unwrap();
        let decoded = decode_datagram(&bytes).unwrap();

        assert_eq!(decoded.node_id, "KY-71AF92");
        assert_eq!(decoded.callsign, "Ana");
    }

    #[test]
    fn rejects_oversized_datagram() {
        let bytes = vec![0_u8; MAX_PACKET_BYTES + 1];
        assert!(decode_datagram(&bytes).is_err());
    }

    #[test]
    fn deduplicator_rejects_seen_packet_ids() {
        let packet_id = Uuid::new_v4();
        let mut dedup = PacketDeduplicator::new(8);

        assert!(dedup.observe(packet_id));
        assert!(!dedup.observe(packet_id));
        assert_eq!(dedup.len(), 1);
    }

    #[test]
    fn deduplicator_evicts_old_ids() {
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let third = Uuid::new_v4();
        let mut dedup = PacketDeduplicator::new(2);

        assert!(dedup.observe(first));
        assert!(dedup.observe(second));
        assert!(dedup.observe(third));
        assert!(dedup.observe(first));
    }
}
