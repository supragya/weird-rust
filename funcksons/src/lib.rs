#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]

//! Let's start our discussion with something called "FUNCTION ITEMS"
//! And how they are related to "FUNCTION POINTERS"
//! Consider the following functions

#[test]
fn func_items_func_pointers() {
    // What really is the type of `f`? If we start with hints,
    // we get `fn fn_none_usize() -> usize`
    // In essence, that means that `f` is some handle to a function
    // that is expected to take zero arguments and return a usize as
    // return.
    // It may look like a function pointer, but it is not. It is a
    // "FUNCTION ITEM", a zero sized handle to a function that is only
    // carried around at the compile time.
    let mut f = fn_none_usize;

    // Consecutively, it is a ZERO SIZED ITEM. you can check it as follows:
    assert_eq!(std::mem::size_of_val(&f), 0);

    // Because of this, even though `randome_function3` has the same
    // signature of arguments and returns as `fn_none_usize`, one cannot
    // really do this... as rust compiler says:
    //
    // Different fn items have unique types, even if their signatures
    // are the same
    //
    // So, the following is not possible

    /* f = fn_none_usize_2; */

    // With generics, FUNCTION POINTERS need expilicit clarity of which
    // `T` we are refering to. The following will not work
    // ERROR: consider specifying the fn_generic argument!

    /* let mut f_generic = fn_generic; */

    // However this works well:
    let f_generic_item = fn_generic::<usize>;

    // However, if we really really cared about this thing to work, we
    // should be able to use function pointers
    let mut f_ptr = fn_none_usize as fn() -> usize;
    f_ptr = fn_none_usize_2;

    // However, this consecutively is not possible
    // ERROR: mismatched types!!

    /* f_ptr = fn_none_u32; */

    // FUNCTION ITEMS and FUNCTION POINTERS are close enough! Infact,
    // fn items can coerce into function pointers when needed!
    fn fn_taker(f_ptr: fn() -> ()) {
        // Function pointers have size
        assert_ne!(std::mem::size_of_val(&f_ptr), 0);
    }
    // Coersion test
    let fn_item = fn_generic::<u32>;
    let fn_item2 = fn_generic::<i128>;
    fn_taker(fn_item);
    fn_taker(fn_item2);
}

fn fn_generic<T>() {}

fn fn_none_usize() -> usize {
    0
}

fn fn_none_u32() -> u32 {
    1
}

fn fn_none_usize_2() -> usize {
    0
}

// -------- FUNCTION AND ITS TRAITS ------------------

// Functions (don't think in terms of items or pointers rn), are self sufficient
// entities. They don't care about state, they don't care about lifetimes etc.
// Also, function items can be coerced into function pointers as need arises
// as we have seen earlier. So, what are function traits? They are there when
// you care about operating on "self".
//      `Fn()` operates on immutable self i.e. &self
//      `FnMut()` operates on mutable self i.e. &mut self
//      `FnOnce()` operates on owned version of self i.e. self
//
// As understable as it is, `Fn()` things can be called anytime, `FnMut()` can
// be called once at a time (relevant across thread boundaries). `FnOnce()` can
// only be invoked once since it "consumes" self.
//
// Here is an interesting bit, just as we have seen FUNCTION ITEMS being coerced
// into FUNCTION POINTERS, the FUNCTION POINTERS implement all three of the above
// traits. a.k.a. `Fn()`, `FnMut()` and `FnOnce()`.
//
// You can trait bound these in a similar fashion.
fn takes_a_function<T>(f: impl Fn()) {}
fn takes_a_function2<T>(f: impl Fn(u32) -> usize) {}

// However, we cannot write this:
/* fn takes_a_function_function<T>(f: impl Fn(Box<impl Fn()>)) {} */

// But why are `Fn` traits required? The answer is CLOSURES

// -------- CLOSURES ----------------

// Closures "close" over their environment. As in, they have an associated
// environment that they "capture". If they wish to not capture their environment
// they are called "non-capturing closures" and may be coercible into function
// pointers.
fn closures_example() {
    let mut some_non_copy_val = String::from("random");

    // This is the simplest, doesn't capture anything. Hence can be coerced into
    // a fn pointer. Hence, implements all three `Fn()`, `FnMut()`, `FnOnce()`
    let non_capturing = || {};

    // All of the following work:
    test_fn_ptr(non_capturing);
    test_fn_trait(non_capturing);
    test_fnmut_trait(non_capturing);
    test_fnonce_trait(non_capturing);

    // Cannot be cast into a function pointer since it requires access to self
    // Implements `Fn()`
    let capturing = || {
        let _ = &some_non_copy_val;
    };

    // All would not work
    // ERROR: closures can only be coerced to `fn` types if they do not
    // capture any variables
    //test_fn_ptr(capturing);
    test_fn_trait(capturing);
    test_fnmut_trait(capturing);
    test_fnonce_trait(capturing);

    // Implements `FnMut()`
    let capturing_mut = || {
        some_non_copy_val.insert(0, 'a');
    };

    // All would not work
    // ERROR: closures can only be coerced to `fn` types if they do not
    // capture any variables
    //test_fn_ptr(capturing_mut);
    //test_fn_trait(capturing_mut);

    // Both of the following work, but can't compile at the same time since they
    // mutate
    test_fnmut_trait(capturing_mut);
    //test_fnonce_trait(capturing_mut);

    // Moves value out of the environment. Implements `FnOnce()`
    let fully_moving = move || {
        let _ = some_non_copy_val;
    };

    // All would not work obviously!
    test_fnonce_trait(fully_moving);
}

fn test_fn_ptr(f: fn()) {}
fn test_fn_trait(f: impl Fn()) {}
fn test_fnmut_trait(f: impl FnMut()) {}
fn test_fnonce_trait(f: impl FnOnce()) {}

// So why do these need `Fn()`, `FnMut()`, `FnOnce()`? Why do these closures
// need `self`? Well, `self` here is needed since closures are "tied to" an
// environment that they work in. They may hold reference to, capture or move
// values from their environment. The compiler would generate some structs
// for this closure which holds these values. And hence these traits come into
// picture

// ------------ NON-STATIC CLOSURES -------------

// Whenever we write:
fn takes_a_fn(f: impl Fn()) {}
// we kind of are implicit that
fn takes_a_fn2(f: impl Fn() + 'static) {}

// But consider the following scenario:
// Here, we are trying to present back a closure which may not live long
// enough if we don't use `move`.
// However, if we use `move`, we can get something that implements `FnOnce`,
// a strictly stronger type. Hence, `move` is necessary
fn function_maker() -> impl Fn() {
    let z = String::from("new_random");
    move || {
        println!("{}", z);
    }
}

// ----------- DYN FN TRAIT OBJECTS --------------

// Much like other trait object that implement some traits and can be presented
// via `Box`, `Fn()`, `FnMut()` and `FnOnce()` can be done similarly, all following
// are correct:

fn dyn_fn1(f: Box<dyn Fn()>) {}
fn dyn_fn2(f: Box<dyn Fn(usize) -> usize>) {}
fn dyn_fn3(f: Box<dyn FnMut(usize)>) {}
fn dyn_fn4(f: Box<dyn FnOnce()>) {}

// Courious case about "unsized locals".
// UNSIZED LOCALS is an ongoing RFC and possibly a nightly-only feature where
// R-Values can be unsized. This is required because when calling `call()` over
// `dyn Fn()`, we may need to do something like this:
//
// ```
// impl FnOnce() for Box<dyn FnOnce()> {
//      fn call(self) {
//          let inner: dyn FnOnce() = self.0; // But what size is it?
//          inner.call()
//      }
// }
// ```

// -------- AN INFO ABOUT INDIRECTION ---------------

// Even when indirection is used, the specific indirection (shared, mutable, owned)
// needs to be taken care of, because
// `&mut dyn FnMut()` makes sense and is callable but `&dyn FnMut()` is not!
// For `FnOnce()`, use `Box`
