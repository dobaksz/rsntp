use crate::error::SynchroniztationError;
use crate::packet::Packet;
use std::net::{ToSocketAddrs, UdpSocket};
use std::time::Duration;

pub trait Transport {
    fn send(&mut self, packet: &Packet) -> Result<(), SynchroniztationError>;
    fn receive(&mut self) -> Result<Packet, SynchroniztationError>;
}

pub struct UdpTransport(UdpSocket);

impl UdpTransport {
    pub fn connect<A: ToSocketAddrs>(addr: A) -> Result<UdpTransport, SynchroniztationError> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(Duration::from_secs(3)))?;
        socket.connect(addr)?;

        Ok(UdpTransport(socket))
    }
}

impl Transport for UdpTransport {
    fn send(&mut self, packet: &Packet) -> Result<(), SynchroniztationError> {
        self.0.send(&packet.encode())?;

        Ok(())
    }

    fn receive(&mut self) -> Result<Packet, SynchroniztationError> {
        let mut buffer = [0; Packet::ENCODED_LEN];

        self.0.recv(&mut buffer)?;

        Ok(Packet::decode(&buffer)?)
    }
}
