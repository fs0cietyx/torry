use async_utp::UtpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn check_api() {
    let mut stream = UtpStream::connect("127.0.0.1:1234").await.unwrap();
    let mut buf = [0u8; 10];
    let _ = stream.read_exact(&mut buf).await;
}
