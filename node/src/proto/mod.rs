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

    pub mod addressed {
        pub use super::super::peerland::message::addressed::message_addressed::*;
        pub use super::super::peerland::message::addressed::MessageAddressed;
    }

    pub mod broadcast {
        pub use super::super::peerland::message::broadcast::message_broadcast::*;
        pub use super::super::peerland::message::broadcast::MessageBroadcast;
    }
}

pub mod time {
    pub mod logical {
        pub use super::super::peerland::time::logical::*;
    }
}
