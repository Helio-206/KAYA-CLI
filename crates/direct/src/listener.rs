use crate::errors::DirectResult;
use tokio::net::{TcpListener, ToSocketAddrs};

pub async fn bind<A>(addr: A) -> DirectResult<TcpListener>
where
    A: ToSocketAddrs,
{
    Ok(TcpListener::bind(addr).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn listener_starts_on_ephemeral_port() {
        let listener = bind("127.0.0.1:0").await.unwrap();
        assert_ne!(listener.local_addr().unwrap().port(), 0);
    }
}
