# korhah [![crates.io](https://img.shields.io/crates/v/korhah.svg)](https://crates.io/crates/korhah) [![docs.rs](https://img.shields.io/docsrs/korhah)](https://docs.rs/korhah)

`korhah` is a minimal & extensible reactive event system.

At its most basic, it allows users to register callback functions and emit _arbitrary_ custom events which will trigger those functions.

It can also act as an object store, whose contents are accessible from within registered callback functions as well as from outside the system.

To keep it minimal, only basic CRUD operations are provided along with signal-y behaviour via automatic dependency-tracking.

To make it extensible, the built-in CRUD operations emit events that can be hooked into.

### Example Usage

```rust
let mut system = korhah::System::default();

// create a simple variable in the reactive system
let a = system.create(|_, _| 0).expect("no cancelling listeners registered");
// create a signal-y variable that depends on `a`
let b = system.create(move |s, _| {
    // the dependency on `a` is automatically tracked here
    let a = s.read(a, |v| *v)
        .expect("no cancelling listeners registered")
        .expect("`a` exists");
    // subsequent updates to `a` will recompute `b` according to this formula
    a + 1
}).expect("no cancelling listeners registered");

// we can emit *any* 'static type we like as an event
struct CustomEvent {
    n: i32,
}

// listen for our custom event being emitted in a "global" scope
// (the "global" scope being due to specifying a `None` target)
system.listen(None, move |s, e: &CustomEvent, _, _| {
    // we'll update `a` to the associated event info
    // (note that this should automatically update `b` too)
    _ = s.update(a, |v| *v = e.n);
});

assert_eq!(Ok(Some(0)), system.read(a, |v| *v));
assert_eq!(Ok(Some(1)), system.read(b, |v| *v));

// emit our custom event
_ = system.emit(None, &CustomEvent { n: 42 });

assert_eq!(Ok(Some(42)), system.read(a, |v| *v));
assert_eq!(Ok(Some(43)), system.read(b, |v| *v));
```

#### `no_std`
This crate is compatible with `no_std` environments, requiring only the `alloc` crate. 

#### (Un)Sync
This crate can be used in both single-threaded and multi-threaded environments.\
In the spirit of [cargo features being additive](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification), the stricter [`Send`](https://doc.rust-lang.org/core/marker/trait.Send.html) + [`Sync`](https://doc.rust-lang.org/core/marker/trait.Sync.html) bounds of a multi-threaded environment are assumed by default for variable types and callback types, and these bounds can be relaxed for single-threaded environments via the `unsync` feature.

#### MSRV
The minimum supported Rust version is **1.63.0**.

---
