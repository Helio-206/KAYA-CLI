use crate::errors::{RelayError, RelayResult};
use kaya_protocol::{decode_with_limit, encode, Packet};
use kaya_shared::MAX_PACKET_BYTES;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const MAX_FRAME_BYTES: usize = MAX_PACKET_BYTES + 4 * 1024;

pub async fn write_packet<W>(writer: &mut W, packet: &Packet) -> RelayResult<usize>
where
    W: AsyncWrite + Unpin,
{
    let bytes = encode(packet).map_err(|err| RelayError::Protocol(err.to_string()))?;
    let len = bytes.len();
    if len > MAX_FRAME_BYTES {
        return Err(RelayError::MalformedFrame(format!(
            "frame exceeds {MAX_FRAME_BYTES} bytes: {len}"
        )));
    }
    writer.write_u32(len as u32).await?;
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(len)
}

pub async fn read_packet<R>(reader: &mut R) -> RelayResult<Packet>
where
    R: AsyncRead + Unpin,
{
    let len = reader.read_u32().await? as usize;
    if len == 0 || len > MAX_FRAME_BYTES {
        return Err(RelayError::MalformedFrame(format!(
            "invalid frame length {len}"
        )));
    }
    let mut buffer = vec![0_u8; len];
    reader.read_exact(&mut buffer).await?;
    decode_with_limit(&buffer, MAX_FRAME_BYTES).map_err(|err| RelayError::Protocol(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaya_protocol::Packet;

    #[tokio::test]
    async fn roundtrips_packet_frame() {
        let packet = Packet::hello("KY-71AF92", "Ana", "geral");
        let mut buffer = tokio::io::duplex(1024);
        let sent = packet.clone();
        let write = tokio::spawn(async move {
            write_packet(&mut buffer.0, &sent).await.unwrap();
        });
        let received = read_packet(&mut buffer.1).await.unwrap();
        write.await.unwrap();

        assert_eq!(received.packet_type, packet.packet_type);
        assert_eq!(received.node_id, packet.node_id);
    }
}
