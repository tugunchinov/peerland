mod dto_cast;

mod peerland {
    pub mod message {
        include!(concat!(env!("OUT_DIR"), "/peerland.message.rs"));
    }

    pub mod lt {
        include!(concat!(env!("OUT_DIR"), "/peerland.lt.rs"));
    }

    pub mod broadcast {
        include!(concat!(env!("OUT_DIR"), "/peerland.broadcast.rs"));
    }
}

pub mod message {
    pub use super::peerland::message::node_message::{Lt, MessageKind, Uuid};
    pub use super::peerland::message::NodeMessage;
}

pub mod lt {
    pub use super::peerland::lt::LamportClockUnit;
}

pub mod broadcast {
    pub use super::peerland::broadcast::broadcast::BroadcastType;
    pub use super::peerland::broadcast::Broadcast;
}
