# Tracker - track changes to structs efficiently

[![Rust](https://github.com/AaronErhardt/Tracker/actions/workflows/rust.yml/badge.svg)](https://github.com/AaronErhardt/Tracker/actions/workflows/rust.yml)
[![Tracker on crates.io](https://img.shields.io/crates/v/tracker)](https://crates.io/crates/tracker)
[![Tracker on docs.rs](https://docs.rs/tracker/badge.svg)](https://docs.rs/tracker)

Tracker is a small crate that allows you to track changes to struct fields.

It implements the following methods for your struct fields:

+ `get_#field_name()`  
  Get an immutable reference to `field_name`

+ `get_mut_#field_name()`  
  Get a mutable reference to `field_name`. Assumes the field will be modified and marks it as changed.

+ `set_#field_name(value)`  
  Set a value of `field_name`. Marks the field as changed only if the new value isn't equal with the previous value.

+ `update_#field_name(fn)`  
  Update your `field_name` with a function or closure. Assumes the field will be modified and marks it as changed.

+ `changed_#field_name()`  
  Check if value of `field_name` has changed.

To check for changes explicitly you can call `var_name.changed(StructName::field_name())` and it will return a bool.
Multiple fields can be checked with `var_name.changed(StructName::field_name_1() | StructName::field_name_2())`.
Finally, it is possible to check for any changes at all with `var_name.changed(StructName::track_all())` or its shortcut
`var_name.changed_any()`.

To reset all previous changes you can call `var_name.reset()`.


## How it works

Let's have a look at a small example.

```rust
#[tracker::track]
struct Test {
    x: u8,
    y: u64,
}

fn main() {
    let mut t = Test {
        x: 0,
        y: 0,
        // the macro generates a new variable called
        // "tracker" that stores the changes
        tracker: 0,
    };

    t.set_x(42);
    // let's check whether the change was detected
    assert!(t.changed(Test::x()));

    // reset t so we don't track old changes
    t.reset();

    t.set_x(42);
    // same value so no change
    assert!(!t.changed(Test::x()));
}
```

What happens behind the scenes when you call `set_x()` is that a bitflag is set in the tracker field of your struct:

```
                                        y   x
tracker: u8 = | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
set_x(42)  -> | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
reset()    -> | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
```

As you can see this works pretty efficient.
The macro expansion looks like this:

```rust
impl Test {
    pub fn get_x(&self) -> &u8 {
        &self.x
    }
    pub fn get_mut_x(&mut self) -> &mut u8 {
        self.tracker |= Self::x();
        &mut self.x
    }
    pub fn update_x<F: Fn(&mut u8)>(&mut self, f: F) {
        self.tracker |= Self::x();
        f(&mut self.x);
    }
    pub const fn x() -> u8 {
        1 << 0usize
    }
    pub fn set_x(&mut self, value: u8) {
        if self.x != value {
        self.tracker |= Self::x();
        }
        self.x = value;
    }
}
```

## Further attributes

```rust
#[tracker::track]
struct Test {
    #[tracker::do_not_track]
    a: u8,
    #[do_not_track]
    b: u8,
    #[tracker::no_eq]
    c: u8,
    #[no_eq]
    d: u8,
}
```

You can mark fields as

+ `do_not_track` if you don't want tracker to implement anything for this field
+ `no_eq` if the type of the field doesn't implement PartialEq or tracker should not check for equality when calling `set_#field_name(value)` 
so that even overwriting with the same value marks the field as changed.
