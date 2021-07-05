use struct_tracker::{tracker, Tracker};

#[derive(Debug, PartialEq)]
enum Test {
    Do,
    Not,
    Copy,
}

impl Default for Test {
    fn default() -> Self {
        Test::Do
    }
}

#[tracker]
#[derive(Default)]
struct Bar {
    x: u8,
    y: u8,
    z: u8,
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    o: Option<u128>,
    test: Test
}

fn main() {
    let mut bar = Bar::default();
    bar.set_y(10);
    assert_eq!(10, *bar.get_y());
    assert!(bar.changed(Bar::y()));
    assert!(bar.changed(Bar::all()));
    bar.reset();
    assert!(!bar.changed(Bar::y()));

    let test = bar.update_test(|test| {
        if *test == Test::Do {
            *test = Test::Not;
        }
    });
    assert_eq!(*bar.get_test(), Test::Not);
}
