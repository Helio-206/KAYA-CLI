use crate::errors::DirectResult;
use tokio::net::TcpStream;

pub async fn connect(addr: &str) -> DirectResult<TcpStream> {
    Ok(TcpStream::connect(addr).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener;

    #[tokio::test]
    async fn connects_to_tcp_listener() {
        let listener = listener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let accept = tokio::spawn(async move { listener.accept().await.unwrap() });

        let stream = connect(&addr.to_string()).await.unwrap();
        let (_, remote_addr) = accept.await.unwrap();

        assert_eq!(stream.peer_addr().unwrap(), addr);
        assert!(remote_addr.ip().is_loopback());
    }
}
