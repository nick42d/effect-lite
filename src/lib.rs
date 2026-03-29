#![no_std]
//! # effect-lite
//! Lightweight (no-std compatible), opinionated algebraic effects library for Rust.
//! ## Goals
//! - No-std compatibility (feature gated).
//! - Demistify the benefits of algebraic effects for those new to functional-style programming.
//! ## Non-goals
//! - Incorporate advanced runtimes or application frameworks - this should be a low-level crate with higher level features built into dependent crates.
//! - Use traditional FP method names at the expense of onboarding difficulty - this is an opinionated crate and I am not a purist.
//! ## Examples
//! ### Dependency injection
//! // An effect that requires access to the inaccessible application state in the parent struct.
//! ```
//! #[cfg(Default)]
//! struct AppState {
//!     count: usize,
//!     component: component::Component,
//! }
//! #[cfg(Default)]
//! mod component {
//!     pub struct Component {
//!         pub message: String
//!     }
//!     impl Component {
//!         pub fn handle_message(&mut self, msg: String) -> impl Effect<()> {
//!             self.message = msg;
//!             once_dependency::<&AppState>().map_output(|app_state| app_state.counter +=1)
//!         }
//!     }
//! }
//! let state = AppState::default();
//! let effect = state.component.handle_message("Hello, world");
//! assert_eq!(&state.component.message, "Hello, world!");
//! assert_eq!(&state.count, 0);
//! effect.resolve(&mut state);
//! assert_eq!(&state.count, 1);
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
//! let effect = once_dependency::<&StdoutPrinter>()
//!     .map_output(|printer| printer.print("Hello, world"));
//! // Prints to console here.
//! effect.resolve(&StdoutPrinter);
//! ```
//! ### Dependency injection - async
//! // An effect that requires access to a `reqwest::Client`.
//! ```
//! let client = reqwest::Client::new();
//! let effect = once_dependency(&client);
//! effect.map_output_async(|client|
//!     client.get("https://www.google.com").send().await.unwrap().text.await.unwrap()
//! );
//! let future = effect.resolve(&client);
//! let output = futures::runtime::block_on(future);
//! println!("{output}");
//! ```
//! ### Module-level access control
//! // An effect that requires access to an inaccessible application state
//! ```
//! #[cfg(Default)]
//! struct AppState {
//!     component: component::Component,
//! }
//! #[cfg(Default)]
//! mod component {
//!     // Note no public setters, so parent modules can't change content...
//!     pub struct Component {
//!         content: String
//!     }
//!     impl Component {
//!         fn update_content(&mut self) -> impl Effect<(&mut Self, &reqwest::Client)> {
//!             // ...but we can provide parent module an Effect that grants the required access.
//!             once_dependency::<(&mut Self, &Client)>
//!                 .map_output_async(|(this, client)|
//!                     let output = client.get("https://www.google.com").send().await.unwrap().text.await.unwrap();
//!                     this.content = output;
//!                 )
//!         }
//!         fn get_content(&self) -> &str {
//!             self.content
//!         }
//!     }
//! }
//! let mut state = AppState::default();
//! let client = reqwest::Client::new();
//! let effect = state.component.update_content();
//! let future = effect.resolve((&mut state.component, &client));
//! futures::runtime::block_on(future);
//! // content should contain the body of Google.com
//! println!("{}", state.component.get_content());
//! ```

pub mod adapters;

pub trait Effect<D> {
    type Output;
    fn resolve(self, dependency: D) -> Self::Output;
}

/// Basic Effect returning the contained value once.
/// ```
/// let x = Once("Hello, world!");
/// assert_eq!(x.into_stream(()).next().await, "Hello, world!");
/// ```
struct Once<T>(T);

impl<T> Effect<()> for Once<T> {
    type Output = T;
    fn resolve(self, _dependency: ()) -> T {
        self.0
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

impl<F, Fut, T> Effect<()> for OnceAsync<F>
where
    // AsyncFnOnce is required here, is I'm unsure how to declare an AsyncFn that does not return a value containing a lifetime.
    F: Fn() -> Fut,
    Fut: Future<Output = T>,
    // F: 'static,
{
    type Output = Fut;
    fn resolve(self, _dependency: ()) -> Self::Output {
        self.0()
    }
}

/// Basic Effect returning the dependency once.
/// ```
/// let x = OnceDependency;
/// assert_eq!(x.into_stream("Hello, world!").next().await, "Hello, world!");
/// ```
struct OnceDependency<D>(core::marker::PhantomData<D>);

fn once_dependency<D>() -> OnceDependency<D> {
    OnceDependency(core::marker::PhantomData)
}
impl<D> Effect<D> for OnceDependency<D> {
    type Output = D;
    fn resolve(self, dependency: D) -> Self::Output {
        dependency
    }
}
