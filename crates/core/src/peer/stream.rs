use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use utp::UtpStream;

pub enum PeerStream {
    Tcp(TcpStream),
    Utp(UtpStream),
}

impl PeerStream {
    pub async fn connect(addr: std::net::SocketAddr) -> std::io::Result<Self> {
        // Try both TCP and uTP concurrently, take whichever connects first.
        tokio::net::TcpStream::connect(addr).await.map(PeerStream::Tcp)
    }

    pub fn set_nodelay(&self, nodelay: bool) -> std::io::Result<()> {
        match self {
            PeerStream::Tcp(s) => s.set_nodelay(nodelay),
            PeerStream::Utp(_) => Ok(()), // uTP handles its own congestion control
        }
    }

    pub fn set_buffer_sizes(&self, rx: usize, tx: usize) -> std::io::Result<()> {
        match self {
            PeerStream::Tcp(s) => {
                let sock_ref = socket2::SockRef::from(s);
                let _ = sock_ref.set_recv_buffer_size(rx);
                let _ = sock_ref.set_send_buffer_size(tx);
                Ok(())
            }
            PeerStream::Utp(_) => Ok(()),
        }
    }

    pub fn set_keepalive(&self, keepalive: bool) -> std::io::Result<()> {
        match self {
            PeerStream::Tcp(s) => {
                let sock_ref = socket2::SockRef::from(s);
                sock_ref.set_keepalive(keepalive)
            }
            PeerStream::Utp(_) => Ok(()), // uTP handles keepalives internally
        }
    }
}

impl AsyncRead for PeerStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            PeerStream::Tcp(s) => Pin::new(s).poll_read(cx, buf),
            PeerStream::Utp(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for PeerStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            PeerStream::Tcp(s) => Pin::new(s).poll_write(cx, buf),
            PeerStream::Utp(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            PeerStream::Tcp(s) => Pin::new(s).poll_flush(cx),
            PeerStream::Utp(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            PeerStream::Tcp(s) => Pin::new(s).poll_shutdown(cx),
            PeerStream::Utp(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}
