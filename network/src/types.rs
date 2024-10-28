#[cfg(not(feature = "simulation"))]
pub type UdpSocket = tokio::net::UdpSocket;
#[cfg(feature = "simulation")]
pub type UdpSocket = turmoil::net::UdpSocket;

#[cfg(not(feature = "simulation"))]
pub use tokio::net::ToSocketAddrs;
#[cfg(feature = "simulation")]
pub use turmoil::ToSocketAddrs;
