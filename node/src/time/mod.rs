mod logic;
mod system;

pub(crate) use logic::lamport_clock::{LamportClock, LamportClockUnit};
pub(crate) use system::unix::UnixTimeProvider;

#[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Millis(pub u128);

impl From<u128> for Millis {
    fn from(val: u128) -> Self {
        Self(val)
    }
}

pub trait SystemTimeProvider: Send + Sync + 'static {
    fn now_millis(&self) -> Millis;
}

pub trait LogicalTimeProvider: Send + Sync + 'static {
    type Unit: Ord + Into<crate::proto::message::Lt>;
    fn new_with_id(id: u32) -> Self;
    fn tick(&self) -> Self::Unit;

    // TODO: add Result
    fn adjust_from_message(&self, message: &crate::proto::message::NodeMessage);
}
