mod dto_cast;

mod peerland {
    include!(concat!(env!("OUT_DIR"), "/peerland.rs"));

    pub mod message {
        pub mod addressed {
            include!(concat!(env!("OUT_DIR"), "/peerland.message.addressed.rs"));
        }

        pub mod broadcast {
            include!(concat!(env!("OUT_DIR"), "/peerland.message.broadcast.rs"));
        }
    }

    pub mod time {
        pub mod logical {
            include!(concat!(env!("OUT_DIR"), "/peerland.time.logical.rs"));
        }
    }
}

pub mod message {
    pub use super::peerland::node_message::*;
    pub use super::peerland::NodeMessage;

    pub use super::peerland::message::addressed;
    pub use super::peerland::message::broadcast;
}

pub mod time {
    pub use super::peerland::time::logical;
}
