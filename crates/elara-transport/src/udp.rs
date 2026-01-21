//! UDP transport implementation

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use elara_core::{ElaraError, ElaraResult};
use elara_wire::{Frame, MAX_FRAME_SIZE};

/// UDP transport for ELARA
pub struct UdpTransport {
    socket: Arc<UdpSocket>,
    local_addr: SocketAddr,
}

impl UdpTransport {
    /// Bind to a local address
    pub async fn bind(addr: SocketAddr) -> ElaraResult<Self> {
        let socket = UdpSocket::bind(addr)
            .await
            .map_err(|e| ElaraError::TransportError(e.to_string()))?;

        let local_addr = socket
            .local_addr()
            .map_err(|e| ElaraError::TransportError(e.to_string()))?;

        Ok(UdpTransport {
            socket: Arc::new(socket),
            local_addr,
        })
    }

    /// Get local address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Send a frame to a destination
    pub async fn send_to(&self, frame: &Frame, dest: SocketAddr) -> ElaraResult<()> {
        let bytes = frame.serialize()?;
        self.socket
            .send_to(&bytes, dest)
            .await
            .map_err(|e| ElaraError::TransportError(e.to_string()))?;
        Ok(())
    }

    /// Send raw bytes to a destination
    pub async fn send_bytes_to(&self, bytes: &[u8], dest: SocketAddr) -> ElaraResult<()> {
        self.socket
            .send_to(bytes, dest)
            .await
            .map_err(|e| ElaraError::TransportError(e.to_string()))?;
        Ok(())
    }

    /// Receive a frame (blocking)
    pub async fn recv_from(&self) -> ElaraResult<(Frame, SocketAddr)> {
        let mut buf = vec![0u8; MAX_FRAME_SIZE];
        let (len, addr) = self
            .socket
            .recv_from(&mut buf)
            .await
            .map_err(|e| ElaraError::TransportError(e.to_string()))?;

        let frame = Frame::parse(&buf[..len])?;
        Ok((frame, addr))
    }

    /// Receive raw bytes (blocking)
    pub async fn recv_bytes_from(&self) -> ElaraResult<(Vec<u8>, SocketAddr)> {
        let mut buf = vec![0u8; MAX_FRAME_SIZE];
        let (len, addr) = self
            .socket
            .recv_from(&mut buf)
            .await
            .map_err(|e| ElaraError::TransportError(e.to_string()))?;

        Ok((buf[..len].to_vec(), addr))
    }

    /// Get a clone of the socket for concurrent operations
    pub fn socket(&self) -> Arc<UdpSocket> {
        Arc::clone(&self.socket)
    }
}

/// Packet receiver channel
pub type PacketReceiver = mpsc::Receiver<(Vec<u8>, SocketAddr)>;

/// Packet sender channel
pub type PacketSender = mpsc::Sender<(Vec<u8>, SocketAddr)>;

/// Start a background receive loop
pub fn start_receive_loop(
    socket: Arc<UdpSocket>,
    buffer_size: usize,
) -> PacketReceiver {
    let (tx, rx) = mpsc::channel(buffer_size);

    tokio::spawn(async move {
        let mut buf = vec![0u8; MAX_FRAME_SIZE];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    let packet = buf[..len].to_vec();
                    if tx.send((packet, addr)).await.is_err() {
                        break; // Receiver dropped
                    }
                }
                Err(e) => {
                    tracing::warn!("UDP receive error: {}", e);
                }
            }
        }
    });

    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_udp_transport_bind() {
        let transport = UdpTransport::bind("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();

        assert_ne!(transport.local_addr().port(), 0);
    }
}
