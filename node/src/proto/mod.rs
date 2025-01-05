mod dto_cast;

pub(crate) mod message {
    include!(concat!(env!("OUT_DIR"), "/message.rs"));
}

pub(crate) mod logical_time {
    include!(concat!(env!("OUT_DIR"), "/logical_time.rs"));
}
