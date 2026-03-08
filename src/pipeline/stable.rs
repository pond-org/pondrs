//! Stable replacements for nightly-only traits.
//!
//! These traits exist as drop-in substitutes for unstable compiler features:
//!
//! - [`StableTuple`] replaces `core::marker::Tuple` (`#![feature(tuple_trait)]`)
//! - [`StableFn`] replaces `Fn<Args>` / `Fn::call` (`#![feature(fn_traits, unboxed_closures)]`)
//!
//! Once the upstream features stabilize, these can be removed and the code
//! switched back to the standard library traits.

mod sealed {
    pub trait Sealed {}
}

/// Marker trait for tuple types.
///
/// Replaces the nightly `core::marker::Tuple`. Sealed so that only tuples
/// can implement it, which is critical for [`IntoNodeResult`] disambiguation.
///
/// [`IntoNodeResult`]: super::IntoNodeResult
pub trait StableTuple: sealed::Sealed {}

impl sealed::Sealed for () {}
impl StableTuple for () {}

macro_rules! impl_stable_tuple {
    ($($T:ident),+) => {
        impl<$($T),+> sealed::Sealed for ($($T,)+) {}
        impl<$($T),+> StableTuple for ($($T,)+) {}
    };
}

impl_stable_tuple!(T0);
impl_stable_tuple!(T0, T1);
impl_stable_tuple!(T0, T1, T2);
impl_stable_tuple!(T0, T1, T2, T3);
impl_stable_tuple!(T0, T1, T2, T3, T4);
impl_stable_tuple!(T0, T1, T2, T3, T4, T5);
impl_stable_tuple!(T0, T1, T2, T3, T4, T5, T6);
impl_stable_tuple!(T0, T1, T2, T3, T4, T5, T6, T7);
impl_stable_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
impl_stable_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);

/// A callable that accepts arguments packed as a tuple.
///
/// Replaces the nightly `Fn<Args>` bound and `Fn::call(&f, args)` syntax.
/// Implemented for all `Fn(T0, T1, ...) -> R` up to 10 arguments.
pub trait StableFn<Args> {
    type Output;
    fn call(&self, args: Args) -> Self::Output;
}

// Zero-argument functions
impl<F, R> StableFn<()> for F
where
    F: Fn() -> R,
{
    type Output = R;
    fn call(&self, _args: ()) -> R {
        (self)()
    }
}

macro_rules! impl_stable_fn {
    ($($T:ident),+) => {
        #[allow(non_snake_case)]
        impl<Func, R, $($T),+> StableFn<($($T,)+)> for Func
        where
            Func: Fn($($T),+) -> R,
        {
            type Output = R;
            fn call(&self, ($($T,)+): ($($T,)+)) -> R {
                (self)($($T),+)
            }
        }
    };
}

impl_stable_fn!(T0);
impl_stable_fn!(T0, T1);
impl_stable_fn!(T0, T1, T2);
impl_stable_fn!(T0, T1, T2, T3);
impl_stable_fn!(T0, T1, T2, T3, T4);
impl_stable_fn!(T0, T1, T2, T3, T4, T5);
impl_stable_fn!(T0, T1, T2, T3, T4, T5, T6);
impl_stable_fn!(T0, T1, T2, T3, T4, T5, T6, T7);
impl_stable_fn!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
impl_stable_fn!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
