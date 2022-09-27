//! # Tracker - track changes to structs efficiently
//!
//! Tracker is a small crate that allows you to track changes to struct fields.
//!
//! It implements the following methods for your struct fields:
//!
//! + `get_#field_name()`
//!   Get a immutable reference to your field
//!
//! + `get_mut_#field_name()`
//!   Get a mutable reference to your field. Assumes the field will be modified and marks it as changed.
//!
//! + `set_#field_name(value)`
//!   Get a mutable reference to your field. Marks the field as changed only if the new value isn't equal with the previous value.
//!
//! + `update_#field_name(fn)`
//!   Update your mutable field with a function or closure. Assumes the field will be modified and marks it as changed.
//!
//! To check for changes you can call `var_name.changed(StructName::field_name())` and it will return a bool.
//!
//! To reset all previous changes you can call `var_name.reset()`.
//!
//!
//! ## How it works
//!
//! Let's have a look at a small example.
//!
//! ```rust
//! #[tracker::track]
//! struct Test {
//!     x: u8,
//!     y: u64,
//! }
//!
//! let mut t = Test {
//!     x: 0,
//!     y: 0,
//!     // the macro generates a new variable called
//!     // "tracker" that stores the changes
//!     tracker: 0,
//! };
//!
//! t.set_x(42);
//! // let's check whether the change was detected
//! assert!(t.changed(Test::x()));
//!
//! // reset t so we don't track old changes
//! t.reset();
//!
//! t.set_x(42);
//! // same value so no change
//! assert!(!t.changed(Test::x()));
//! ```
//!
//! What happens behind the scenes when you call `set_x()` is that a bitflag is set in the tracker field of your struct:
//!
//! ```ignore
//!                                         y   x
//! tracker: u8 = | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
//! set_x(42)  -> | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
//! reset()    -> | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
//! ```
//!
//! As you can see this works pretty efficient.
//! The macro expansion looks like this:
//!
//! ```
//! # struct Test {
//! #     x: u8,
//! #     tracker: u8,
//! # }
//! #
//! impl Test {
//!     pub fn get_x(&self) -> &u8 {
//!         &self.x
//!     }
//!     pub fn get_mut_x(&mut self) -> &mut u8 {
//!         self.tracker |= Self::x();
//!         &mut self.x
//!     }
//!     pub fn update_x<F: Fn(&mut u8)>(&mut self, f: F) {
//!         self.tracker |= Self::x();
//!         f(&mut self.x);
//!     }
//!     pub const fn x() -> u8 {
//!         1 << 0usize
//!     }
//!     pub fn set_x(&mut self, value: u8) {
//!         if self.x != value {
//!         self.tracker |= Self::x();
//!         }
//!         self.x = value;
//!     }
//! }
//! ```
//!
//! ## Further attributes
//!
//! ```rust
//! #[tracker::track]
//! struct Test {
//!     #[tracker::do_not_track]
//!     a: u8,
//!     #[do_not_track]
//!     b: u8,
//!     #[tracker::no_eq]
//!     c: u8,
//!     #[no_eq]
//!     d: u8,
//! }
//! ```
//!
//! You can mark fields as
//!
//! + `do_not_track` if you don't want tracker to implement anything for this field
//! + `no_eq` if the type of the field doesn't implement PartialEq or tracker should not check for equality when calling `set_#field_name(value)`
//! so that even overwriting with the same value marks the field as changed.
//! pub use tracker_macros::track;

#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub,
    clippy::cargo,
    clippy::must_use_candidate
)]

pub use tracker_macros::track;

#[cfg(test)]
mod test {

    #[derive(Debug, PartialEq)]
    enum NoCopy {
        Do,
        Not,
        _Copy,
    }

    impl Default for NoCopy {
        fn default() -> Self {
            NoCopy::Do
        }
    }

    #[crate::track]
    struct TestDefaultParam<Config, Allocator = NoCopy>
    where
        Config: ?Sized + std::fmt::Debug,
    {
        _config: std::marker::PhantomData<Config>,
        _allocator: std::marker::PhantomData<Allocator>,
    }

    const _TEST_WHERE: TestDefaultParam<[u8]> = TestDefaultParam {
        _config: std::marker::PhantomData,
        _allocator: std::marker::PhantomData,
        tracker: 0,
    };

    #[crate::track]
    struct Empty {}

    #[crate::track]
    #[derive(Default)]
    struct Test {
        x: u8,
        y: u8,
        _z: u8,
        #[tracker::do_not_track]
        a: u8,
        b: u8,
        #[tracker::no_eq]
        c: u8,
        _d: u8,
        #[do_not_track]
        _e: u8,
        _o: Option<u128>,
        #[no_eq]
        no_copy: NoCopy,
    }

    #[crate::track]
    struct Generic<T: std::fmt::Debug> {
        #[no_eq]
        test: T,
        int: u8,
    }

    #[test]
    fn test_all() {
        let mut empty = Empty { tracker: 1 };
        assert!(empty.changed(Empty::track_all()));
        empty.reset();

        let mut t = Test::default();
        t.set_b(10);
        assert_eq!(10, *t.get_b());
        // b should be 2^3 because a is ignored
        assert!(t.changed(8));

        t.set_c(10);
        assert!(t.changed(Test::c()));

        t.reset();

        // b and c are already 10. But only for b eq is checked and no change bit is set.
        t.set_b(10);
        t.set_c(10);
        assert!(!t.changed(Test::b()));
        assert!(t.changed(Test::c()));
        assert!(t.changed(Test::track_all()));

        t.reset();

        t.update_no_copy(|no_copy| {
            *no_copy = NoCopy::Not;
        });
        assert_eq!(*t.get_no_copy(), NoCopy::Not);
        assert!(t.changed(Test::no_copy()));

        t.get_x();
        assert!(!t.changed(Test::x()));

        t.get_mut_y();
        assert!(t.changed(Test::y()));

        t.reset();

        t.a = 10;
        assert!(!t.changed(Test::track_all()));

        let mut g = Generic {
            test: 0u8,
            int: 1,
            tracker: 0,
        };

        g.set_test(1);
        assert!(g.changed(Generic::<u8>::test()));
    }
}
