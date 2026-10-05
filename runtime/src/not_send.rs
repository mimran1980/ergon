//! `assert_not_send!`: a compile-time proof that a type is not `Send`, with
//! no dependency. A trait has two blanket impls, the second only for `Send`
//! types, so naming its item for a `Send` type is ambiguous (E0283) and does
//! not compile. `tests/client_ban_lint.rs` compiles this file with a `Send`
//! type and requires that failure.

/// Fails to compile if any of the types is `Send`.
macro_rules! assert_not_send {
    ($($t:ty),+ $(,)?) => {
        $(
            const _: fn() = || {
                trait AmbiguousIfSend<A> {
                    fn some_item() {}
                }
                impl<T: ?Sized> AmbiguousIfSend<()> for T {}
                impl<T: ?Sized + Send> AmbiguousIfSend<u8> for T {}
                let _ = <$t as AmbiguousIfSend<_>>::some_item;
            };
        )+
    };
}
