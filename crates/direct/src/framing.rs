use crate::errors::{DirectError, DirectResult};
use kaya_protocol::{decode_with_limit, encode, Packet};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const MAX_DIRECT_FRAME_BYTES: usize = 8 * 1024 * 1024;

pub async fn write_packet<W>(writer: &mut W, packet: &Packet) -> DirectResult<usize>
where
    W: AsyncWrite + Unpin,
{
    let bytes = encode(packet).map_err(|err| DirectError::Protocol(err.to_string()))?;
    let len = bytes.len();
    if len > MAX_DIRECT_FRAME_BYTES {
        return Err(DirectError::MalformedFrame(format!(
            "frame exceeds {MAX_DIRECT_FRAME_BYTES} bytes: {len}"
        )));
    }
    writer.write_u32(len as u32).await?;
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(len)
}

pub async fn read_packet<R>(reader: &mut R) -> DirectResult<Packet>
where
    R: AsyncRead + Unpin,
{
    read_packet_frame(reader).await.map(|(packet, _)| packet)
}

pub async fn read_packet_frame<R>(reader: &mut R) -> DirectResult<(Packet, usize)>
where
    R: AsyncRead + Unpin,
{
    let len = reader.read_u32().await? as usize;
    if len == 0 || len > MAX_DIRECT_FRAME_BYTES {
        return Err(DirectError::MalformedFrame(format!(
            "invalid frame length {len}"
        )));
    }
    let mut buffer = vec![0_u8; len];
    reader.read_exact(&mut buffer).await?;
    let packet = decode_with_limit(&buffer, MAX_DIRECT_FRAME_BYTES)
        .map_err(|err| DirectError::Protocol(err.to_string()))?;
    Ok((packet, len + 4))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaya_protocol::Packet;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn roundtrips_packet_frame() {
        let packet = Packet::hello("KY-71AF92", "Ana", "geral");
        let sent = packet.clone();
        let mut stream = tokio::io::duplex(4096);

        let write = tokio::spawn(async move {
            write_packet(&mut stream.0, &sent).await.unwrap();
        });
        let received = read_packet(&mut stream.1).await.unwrap();
        write.await.unwrap();

        assert_eq!(received.packet_type, packet.packet_type);
        assert_eq!(received.node_id, packet.node_id);
    }

    #[tokio::test]
    async fn rejects_invalid_frame_length() {
        let mut stream = tokio::io::duplex(16);
        let write = tokio::spawn(async move {
            stream.0.write_u32(0).await.unwrap();
        });

        let err = read_packet(&mut stream.1).await.unwrap_err();
        write.await.unwrap();

        assert!(matches!(err, DirectError::MalformedFrame(_)));
    }
}
