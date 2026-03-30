#![no_std]
//! # effect-lite
//! An experimental, lightweight (no-std compatible), opinionated algebraic effects library for Rust.
//! ## Goals
//! - No-std compatibility (feature gated).
//! - Use zero-cost abstractions.
//! - Easy onboarding.
//! ## Non-goals
//! - Incorporate advanced runtimes or application frameworks - this should be a low-level crate with higher level features built into dependent crates.
//! - Use traditional FP method names at the expense of onboarding difficulty - this is an opinionated crate and I am not a purist.
//! ## Examples
//! ### Inversion of control
//! // An effect that requires access to the inaccessible application state in the parent struct.
//! ```
//! use effect_light::Effect;
//!
//! #[derive(Default)]
//! struct AppState {
//!     count: usize,
//!     component: component::Component,
//! }
//! mod component {
//!     #[derive(Default)]
//!     pub struct Component {
//!         pub message: String
//!     }
//!     impl Component {
//!         pub fn handle_message(&mut self, msg: String) -> impl effect_light::Effect<&mut super::AppState> + 'static {
//!             self.message = msg;
//!             effect_light::fn_effect(|app_state: &mut super::AppState| app_state.count +=1)
//!         }
//!     }
//! }
//! # // Defer main to ensure component module can use `super::` reference.
//! # fn main() {
//! let mut state = AppState::default();
//! let effect = state.component.handle_message("Hello, world!".to_string());
//! assert_eq!(state.component.message, "Hello, world!");
//! assert_eq!(state.count, 0);
//! effect.resolve(&mut state);
//! assert_eq!(state.count, 1);
//! # }
//! ```
//! ### Dependency injection
//! // Define generic effects, and resolve them using any type that meets the trait bounds.
//! ```
//! use effect_light::Effect;
//!
//! trait Foo {}
//! struct ProdFoo;
//! struct TestFoo;
//! impl Foo for ProdFoo {}
//! impl Foo for TestFoo {}
//!
//! struct FooEffect;
//! impl<F: Foo> Effect<&F> for FooEffect {
//!     type Output = ();
//!     fn resolve(self, dependency: &F) {}
//! }
//!
//! let (prod, test) = (ProdFoo, TestFoo);
//! // FooEffect can be resolved against both the test and prod 'foo'.
//! FooEffect.resolve(&prod);
//! FooEffect.resolve(&test);
//! ```
//! ### Centralise side effects
//! ```
//! use effect_light::Effect;
//!
//! struct StdoutPrinter;
//! impl StdoutPrinter {
//!     fn print(&self, text: &str) {
//!         println!("{text}");
//!     }
//! }
//! let printer = StdoutPrinter;
//! let effect = effect_light::fn_effect(|printer: &StdoutPrinter| printer.print("Hello, world"));
//! // Prints to console here.
//! effect.resolve(&StdoutPrinter);
//! ```
//! ### Dependency injection - async
//! // An effect that requires access to a `reqwest::Client`.
//! ```
//! use effect_light::Effect;
//!
//! let client = reqwest::Client::new();
//! let effect = effect_light::fn_effect_async(async |client: &reqwest::Client|
//!     client.get("https://www.google.com").send().await.unwrap().text().await.unwrap()
//! );
//! let future = effect.resolve(&client);
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! let output = rt.block_on(future);
//! println!("{output}");
//! ```
//! ### Module-level access control
//! // An effect that requires access to an inaccessible application state
//! ```
//! use effect_light::{Effect, EffectExt};
//!
//! #[derive(Default)]
//! struct AppState {
//!     component: component::Component,
//! }
//! mod component {
//!     // Note no public setters, so parent modules can't change content...
//!     #[derive(Default)]
//!     pub struct Component {
//!         content: String
//!     }
//!     impl Component {
//!         pub fn update_content<'a>() -> impl effect_light::Effect<(&'a mut Self, &'a reqwest::Client), Output=impl Future<Output = ()>>  {
//!             // ...but we can provide parent module an Effect that grants the required access.
//!             effect_light::fn_effect_async(|(this, client): (&mut Self, &reqwest::Client)| async move {
//!                 let output = client.get("https://www.google.com").send().await.unwrap().text().await.unwrap();
//!                 this.content = output;
//!             })
//!         }
//!         pub fn get_content(&self) -> &str {
//!             &self.content
//!         }
//!     }
//! }
//! let mut state = AppState::default();
//! let client = reqwest::Client::new();
//! let effect = component::Component::update_content().map_dependency(|(this, client): (&mut AppState, _)| (&mut this.component, client));
//! let future = effect.resolve((&mut state, &client));
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(future);
//! // content should contain the body of Google.com
//! println!("{}", state.component.get_content());
//! ```

pub use adapters::EffectExt;

pub mod adapters;
pub mod either;

pub trait Effect<D> {
    type Output;
    fn resolve(self, dependency: D) -> Self::Output;
}

/// Effect that produces no output and has no side effects.
pub struct None();
/// Effect that produces no output and has no side effects.
/// ```
/// use effect_light::Effect;
///
/// let mut x = String::new();
/// assert_eq!(effect_light::none().resolve(&mut x), ());
/// assert_eq!(x, String::new());
/// ```
pub fn none() -> None {
    None()
}
impl<D> Effect<D> for None {
    type Output = ();
    fn resolve(self, _dependency: D) {}
}

/// An effect built from a closure.
pub struct FnEffect<T>(T);
/// An effect built from a closure.
/// ```
/// use effect_light::Effect;
///
/// let x = effect_light::fn_effect(|t: &mut Vec<_>| t.pop());
/// let mut y = vec![1,2,3];
/// assert_eq!(x.resolve(&mut y), Some(3));
/// assert_eq!(y, vec![1,2]);
/// ```
pub fn fn_effect<F, D, T>(f: F) -> FnEffect<F>
where
    F: Fn(D) -> T,
{
    FnEffect(f)
}
impl<F, D, T> Effect<D> for FnEffect<F>
where
    F: Fn(D) -> T,
{
    type Output = T;
    fn resolve(self, dependency: D) -> T {
        self.0(dependency)
    }
}

/// An effect built from an async closure.
pub struct AsyncFnEffect<T>(T);
/// An effect built from an async closure.
/// ```
/// use effect_light::Effect;
///
/// let x = effect_light::fn_effect(|t: &mut Vec<_>| t.pop());
/// let mut y = vec![1,2,3];
/// assert_eq!(x.resolve(&mut y), Some(3));
/// assert_eq!(y, vec![1,2]);
/// ```
pub fn fn_effect_async<F, Fut, D, T>(f: F) -> AsyncFnEffect<F>
where
    F: Fn(D) -> Fut,
    Fut: Future<Output = T>,
{
    AsyncFnEffect(f)
}
impl<F, Fut, D, T> Effect<D> for AsyncFnEffect<F>
where
    F: Fn(D) -> Fut,
    Fut: Future<Output = T>,
{
    type Output = Fut;
    fn resolve(self, dependency: D) -> Fut {
        self.0(dependency)
    }
}

/// Basic Effect returning the contained value immediately.
pub struct Value<T>(T);
/// Basic Effect returning the contained value immediately.
/// ```
/// use effect_light::Effect;
///
/// let x = effect_light::value("Hello, world!");
/// assert_eq!(x.resolve(()), "Hello, world!");
/// ```
pub fn value<T>(t: T) -> Value<T> {
    Value(t)
}
impl<T> Effect<()> for Value<T> {
    type Output = T;
    fn resolve(self, _dependency: ()) -> T {
        self.0
    }
}

/// Basic Effect returning the contained value asynchronously.
pub struct AsyncValue<T>(T);
/// Basic Effect returning the contained value asynchronously.
/// ```
/// use effect_light::Effect;
///
/// # futures::executor::block_on(async {
/// let x = effect_light::value_async(async {"Hello, world!"});
/// assert_eq!(x.resolve(()).await, "Hello, world!");
/// # });
/// ```
pub fn value_async<F, T>(f: F) -> AsyncValue<F>
where
    F: Future<Output = T>,
{
    AsyncValue(f)
}
impl<F, T> Effect<()> for AsyncValue<F>
where
    F: Future<Output = T>,
{
    type Output = F;
    fn resolve(self, _dependency: ()) -> Self::Output {
        self.0
    }
}

/// Basic Effect returning the dependency.
/// ```
/// use effect_light::Effect;
///
/// let x = effect_light::echo();
/// assert_eq!(x.resolve("Hello, world!"), "Hello, world!");
/// ```
pub struct Echo<D>(core::marker::PhantomData<D>);
pub fn echo<D>() -> Echo<D> {
    Echo(core::marker::PhantomData)
}
impl<D> Effect<D> for Echo<D> {
    type Output = D;
    fn resolve(self, dependency: D) -> Self::Output {
        dependency
    }
}
