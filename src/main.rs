use struct_tracker::{tracker, Tracker};

#[tracker]
#[derive(Default)]
struct Bar {
    x: u8,
    y: u8,
    f: u8,
    h: u8,
    k: u8,
    l: u8,
    j: u8,
    o: u8,
    test: Option<u128>,
}

fn main() {
    let mut bar = Bar::default();
    bar.set_y(10);
    assert_eq!(10, bar.get_y());
    assert!(bar.changed(Bar::y()));
    assert!(bar.changed(Bar::all()));
    bar.reset();
    assert!(!bar.changed(Bar::y()));
}
