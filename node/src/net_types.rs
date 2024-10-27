#[cfg(not(test))]
pub(crate) type UdpSocket = tokio::net::UdpSocket;
#[cfg(test)]
pub(crate) type UdpSocket = turmoil::net::UdpSocket;

#[cfg(not(test))]
pub(crate) use tokio::net::ToSocketAddrs;
#[cfg(test)]
pub(crate) use turmoil::ToSocketAddrs;
