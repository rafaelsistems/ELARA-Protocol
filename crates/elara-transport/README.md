# elara-transport

ELARA Protocol transport layer: UDP and STUN for NAT traversal.

Part of [ELARA Protocol](https://github.com/rafaelsistems/ELARA-Protocol).

## Usage

```rust
use elara_transport::{UdpTransport, StunClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = UdpTransport::bind("127.0.0.1:0".parse()?).await?;
    transport.send_bytes_to(b"hello", "127.0.0.1:8080".parse()?).await?;
    let (data, from) = transport.recv_bytes_from().await?;
    Ok(())
}
```

## License

MIT OR Apache-2.0
