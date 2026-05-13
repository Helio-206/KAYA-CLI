use kaya_protocol::{decode, encode, Packet};
use kaya_shared::{KayaError, Result, MAX_PACKET_BYTES, MULTICAST_IPV4, MULTICAST_PORT};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{debug, trace};

#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub multicast_ip: Ipv4Addr,
    pub port: u16,
    pub loopback: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            multicast_ip: MULTICAST_IPV4.parse().expect("valid multicast address"),
            port: MULTICAST_PORT,
            loopback: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MulticastTransport {
    socket: Arc<UdpSocket>,
    multicast_addr: SocketAddr,
}

impl MulticastTransport {
    pub async fn bind(config: TransportConfig) -> Result<Self> {
        let socket = create_multicast_socket(&config)?;
        let socket = UdpSocket::from_std(socket)?;
        let multicast_addr = SocketAddr::V4(SocketAddrV4::new(config.multicast_ip, config.port));
        debug!(%multicast_addr, "kaya multicast transport bound");
        Ok(Self {
            socket: Arc::new(socket),
            multicast_addr,
        })
    }

    pub async fn bind_default() -> Result<Self> {
        Self::bind(TransportConfig::default()).await
    }

    pub async fn send_packet(&self, packet: &Packet) -> Result<usize> {
        let bytes = encode(packet)?;
        let sent = self.socket.send_to(&bytes, self.multicast_addr).await?;
        trace!(packet_id = %packet.packet_id, packet_type = ?packet.packet_type, bytes = sent, "packet sent");
        Ok(sent)
    }

    pub async fn recv_packet(&self) -> Result<(Packet, SocketAddr)> {
        let mut buffer = vec![0_u8; MAX_PACKET_BYTES];
        let (len, addr) = self.socket.recv_from(&mut buffer).await?;
        let packet = decode(&buffer[..len])?;
        trace!(packet_id = %packet.packet_id, packet_type = ?packet.packet_type, %addr, "packet received");
        Ok((packet, addr))
    }
}

fn create_multicast_socket(config: &TransportConfig) -> Result<std::net::UdpSocket> {
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

pub fn decode_datagram(bytes: &[u8]) -> Result<Packet> {
    if bytes.len() > MAX_PACKET_BYTES {
        return Err(KayaError::InvalidPacket(format!(
            "packet exceeds {} bytes",
            MAX_PACKET_BYTES
        )));
    }
    decode(bytes)
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
}
