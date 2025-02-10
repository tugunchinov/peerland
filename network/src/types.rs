pub type SocketAddr = std::net::SocketAddr;

#[cfg(not(feature = "simulation"))]
pub use tokio::net::*;
#[cfg(feature = "simulation")]
pub use turmoil::net::*;

#[cfg(not(feature = "simulation"))]
pub type TcpStream = tokio::net::TcpStream;
#[cfg(feature = "simulation")]
pub type TcpStream = turmoil::net::TcpStream;

#[cfg(not(feature = "simulation"))]
pub use tokio::net::ToSocketAddrs;
#[cfg(feature = "simulation")]
pub use turmoil::ToSocketAddrs;
