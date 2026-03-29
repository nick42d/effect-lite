#![no_std]
//! # EffectX
//! ## Examples
//! ### Dependency injection
//! // An effect that requires access to a `reqwest::Client` to run.
//! ```
//! let client = reqwest::Client::new();
//! let effect = once_dependency(&client);
//! effect.map_output(|client|
//!     client.get("https://www.google.com").send().await.unwrap().text.await.unwrap()
//! );
//! let output = runtime::to_iter(&client, effect).next().unwrap();
//! println!("{output}");
//! ```
//! ### Centralise side effects
//! ```
//! struct StdoutPrinter;
//! impl StdoutPrinter {
//!     fn print(&self, text: &str) {
//!         println!("{text}");
//!     }
//! }
//! let printer = StdoutPrinter;
//! let effect = once_dependency(&printer).map_output(|printer| printer.print("Hello, world"));
//! // Prints to console here.
//! runtime::to_iter(&printer, effect).next().unwrap();
//! ```

use futures::{
    future::Either,
    stream::{self, select},
    Stream, StreamExt,
};

pub mod adapters;

pub trait Effect<D> {
    type Output;
    fn into_stream(self, dependency: D) -> impl Stream<Item = Self::Output>;
}

/// Basic Effect returning the contained value once.
/// ```
/// let x = Once("Hello, world!");
/// assert_eq!(x.into_stream(()).next().await, "Hello, world!");
/// ```
struct Once<T>(T);

impl<T> Effect<()> for Once<T> {
    type Output = T;
    fn into_stream(self, dependency: ()) -> impl Stream<Item = Self::Output> {
        stream::once(async { self.0 })
    }
}

/// Basic Effect that runs asynchronously with one output value.
/// ```
/// let x = OnceAsync(async {"Hello, world!"});
/// assert_eq!(x.into_stream(()).next().await, "Hello, world!");
/// ```
pub struct OnceAsync<T>(T);

fn once_async<T, F: AsyncFn() -> T>(f: F) -> OnceAsync<F> {
    OnceAsync(f)
}

impl<F, T> Effect<()> for OnceAsync<F>
where
    // AsyncFnOnce is required here, is I'm unsure how to declare an AsyncFn that does not return a value containing a lifetime.
    F: AsyncFnOnce() -> T,
    F: 'static,
{
    type Output = T;
    fn into_stream(self, _dependency: ()) -> impl Stream<Item = Self::Output> {
        let OnceAsync(f) = self;
        let f = f();
        stream::once(f)
    }
}

/// Basic Effect returning the dependency once.
/// ```
/// let x = OnceDependency;
/// assert_eq!(x.into_stream("Hello, world!").next().await, "Hello, world!");
/// ```
struct OnceDependency;

impl<D> Effect<D> for OnceDependency {
    type Output = D;
    fn into_stream(self, dependency: D) -> impl Stream<Item = Self::Output> {
        stream::once(async { dependency })
    }
}

#[cfg(test)]
mod tests {
    use crate::Effect;

    #[test]
    fn test() {
        struct FetchesNetworkEffect;
        impl Effect<()> for FetchesNetworkEffect {
            type Output = ();
            fn into_stream(self, dependency: ()) -> impl futures::Stream<Item = Self::Output> {
                futures::stream::once(async { |x| x + 1 })
            }
        }
        let x = FetchesNetworkEffect;
    }
}
