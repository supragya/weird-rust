//! Rust has a "Share-able mutable containers" and that is called
//! a CELL, most importantly, we find it in this location:
//!         `std::cell`
//! ref: https://doc.rust-lang.org/std/cell/
//! 
//! Rust compiler enforces for every single object `O`, there can only be:
//! - Several immutable references to `O` via `&O` (aliasing)
//! - One mutable reference to `O` via `&mut O`
//! 
//! Generally, you cannot mutate by having immutable reference, but some containers
//! allow this to be possible in a controlled way. They would be:
//! 
//! - `Cell<T: ?Sized>`: For simple types (small, `Copy` impls)
//!     - `set`: Replaces inner value dropping replaced value
//!     - `replace`: Replaces inner value, returning the replaced value
//!     - `into_inner`: Consumes cell and returns inner value
//!     - `get`: If `Copy` is implemented, provides duplicate
//! 
//! - `RefCell<T: ?Sized>`: For rust lifetimes and dynamic borrow. (not `Sync`, use `RwLock<T>`)
//!     - We can do `.borrow()` or `.borrow_mut()` to emulate it
//! 
//! - `OneCell<T>`: One time set-able type, (not `Sync`, use `OneLock<T>`)
//! 
//! Aside: Another important interior-mutability data structure is `Mutex<T>`
//! and it is in `sync::Mutex` because it depends on some synchronization primitives
//! provided by OS and CPU
//! 
//! Going from `Cell` to `RefCell` to `Mutex` finally, you find yourself "free-er" and
//! can do more stuff. But there's always cost attached to it. Basically, `Cell` is simpler
//! doesn't do bookkeeping, `RefCell` does it, but still minimal and doesn't have to use
//! sync primitives, while `Mutex` uses `Sync` primitives under the hood.
//! 
//! At the core of all interior mutability in rust is `std::cell::UnsafeCell`
//! 
//! IMPORTANT
//! ----------
//! YOU CAN'T KNOW FROM OUTSIDE IF THE TYPE WITHIN IT EXERCISES INTERIOR MUTABILITY


/// An aside on `UnsafeCell`
/// -----------------------
/// `UnsafeCell` is the core primitive of interior mutability in Rust.
/// 
/// If you have a reference &T, then normally in Rust the compiler performs optimizations 
/// based on the knowledge that &T points to immutable data. Mutating that data, 
/// for example through an alias or by transmuting an &T into an &mut T, 
/// is considered undefined behavior. UnsafeCell<T> opts-out of the immutability guarantee 
/// for &T: a shared reference &UnsafeCell<T> may point to data that is being mutated.
/// 
/// All other types that allow internal mutability, such as Cell<T> and RefCell<T>, 
/// internally use UnsafeCell to wrap their data.
/// 
/// The UnsafeCell API itself is technically very simple: `.get()` gives you a raw pointer 
/// *mut T to its contents. It is up to you as the abstraction designer to use that 
/// raw pointer correctly.
/// 
/// Interestingly, `UnsafeCell` is `!Sync`. That means, it cannot be used across thread
/// boundaries. This would become useful soon.


// ----- Let's try building `Cell` ------

/// Remember, `T` can be `?Sized`, may be sized, may not be sized.
/// Also, `Cell` becomes `!Sync` automagically since `UnsafeCell` is `!Sync`.
/// Why do we care about `!Sync`? Because of `.set()` method.
pub struct Cell<T: ?Sized> {
    value: std::cell::UnsafeCell<T>,
}

// implied by having `UnsafeCell`
// impl<T> !Sync for Cell<T> {}
//
// But how do I force compiler to accept `Cell<T>` to be `Sync` when it has
// `!Sync` items within it?
//
// unsafe impl<T> Sync for Cell<T> {}

impl<T> Cell<T> {
    pub fn new(value: T) -> Self {
        Cell { value: std::cell::UnsafeCell::new(value) }
    }

    /// Look here carefully, we have shared reference to `self`, but we want
    /// to mutate it.
    pub fn set(&self, value: T) {
        // `UnsafeCell` gives us the ability to get the raw pointer to the internal
        // type. But we can't set that directly, running following code would fail!

        /*        *self.value.get() = value;      */
        // Error: this operation is unsafe and requires an unsafe function or blockrust-analyzerE0133
        //        dereference of raw pointer is unsafe and requires unsafe function or block
        //        raw pointers may be null, dangling or unaligned; they can violate aliasing 
        //        rules and cause data races: all of these are undefined behavior

        // We have to check that this is indeed safe, and then do unsafe code as follows:

        // SAFETY: we know no-one else is concurrently mutating self.value (because !Sync)
        // SAFETY: we know we are not invalidating any references, because we never give any out
        unsafe { *self.value.get() = value; }
    }

    pub fn get(&self) -> T
    where T: Copy {
        // SAFETY: we know no-one else is modifying this value, since only this thread can mutate
        // (because !Sync) and it is here, getting the value
        unsafe { *self.value.get() }
    }

    // TODO: How to implement `replace`
}

// ----- Let's try building `RefCell` ------

/// Each `RefCell` can be either be:
/// - `Unshared`: No-one shares this RefCell,
/// - `Shared(usize)`: 1 or more folks who hold shared reference
/// - `Exclusive`: Single folk holding an exclusive reference
/// 
/// Why Clone? `Copy` needs `Clone`
#[derive(Copy, Clone)]
pub enum SharingState {
    Unshared,
    Exclusive,
    Shared(usize),
}

pub struct RefCell<T> {
    // This is a no-brainer. RefCell requires us to have a "shared immutable container"
    // so `UnsafeCell` is needed 
    value: std::cell::UnsafeCell<T>,
    // But why is this of type `Cell`? Well, because during `.borrow()` and `.borrow_mut()`,
    // we would like to affect this specific "state". Hence, this in turn requires interior
    // mutability. Question would be why not `UnsafeCell` directly? Well, we can, but we
    // would like to reuse our code above :) 
    sharing_state: Cell<SharingState>,
}

impl<T> RefCell<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: std::cell::UnsafeCell::new(value),
            sharing_state: Cell::new(SharingState::Unshared),
        }
    }

    pub fn borrow(&self) -> Option<&T> {
        match self.sharing_state.get() {
            SharingState::Exclusive => None,
            SharingState::Unshared => {
                self.sharing_state.set(SharingState::Shared(1));
                // SAFETY: no exclusive or aliases have been given out since state is unshared till now,
                // so it is okay to give out an alias / shared reference to the underlying
                unsafe { Some(&*self.value.get()) }
            },
            SharingState::Shared(n) => {
                self.sharing_state.set(SharingState::Shared(n+1));
                // SAFETY: `n` shared references have been given out and what is being asked is yet another
                // shared reference. Hence, it should be okay to give out another shared reference since no
                // one holds an exclusive reference.
                unsafe { Some(&*self.value.get()) }
            }
        }
    }

    pub fn borrow_mut(&self) -> Option<&mut T> {
        match self.sharing_state.get() {
            SharingState::Shared(_) => None,
            SharingState::Exclusive => None,
            SharingState::Unshared => {
                self.sharing_state.set(SharingState::Exclusive);
                // SAFETY: No shared references have been given out. Hence, it should be okay to make an
                // Exclusive reference and give it out. It is checked in the `.borrow()` code that when
                // `SharingState::Exclusive` occurs, no immuatble borrows are given out!
                unsafe { Some(&mut *self.value.get() )}
            },
        }
    }
}
