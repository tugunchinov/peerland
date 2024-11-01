#[cfg(not(feature = "simulation"))]
pub type UdpSocket = tokio::net::UdpSocket;
#[cfg(feature = "simulation")]
pub type UdpSocket = turmoil::net::UdpSocket;

#[cfg(not(feature = "simulation"))]
pub type SocketAddr = tokio::net::unix::SocketAddr;
#[cfg(feature = "simulation")]
pub type SocketAddr = turmoil::net::SocketAddr;

#[cfg(not(feature = "simulation"))]
pub type TcpListener = tokio::net::TcpListener;
#[cfg(feature = "simulation")]
pub type TcpListener = turmoil::net::TcpListener;

#[cfg(not(feature = "simulation"))]
pub type TcpStream = tokio::net::TcpStream;
#[cfg(feature = "simulation")]
pub type TcpStream = turmoil::net::TcpStream;

#[cfg(not(feature = "simulation"))]
pub use tokio::net::ToSocketAddrs;
#[cfg(feature = "simulation")]
pub use turmoil::ToSocketAddrs;
