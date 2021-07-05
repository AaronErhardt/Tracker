pub use struct_tracker_macros::tracker;

pub trait Tracker {
    fn reset(&mut self);
}
