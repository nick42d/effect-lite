# effect-light

## effect-lite
An experimental, lightweight (no-std compatible), opinionated algebraic effects library for Rust.
### Goals
- No-std compatibility (feature gated).
- Use zero-cost abstractions.
- Easy onboarding.
### Non-goals
- Incorporate advanced runtimes or application frameworks - this should be a low-level crate with higher level features built into dependent crates.
- Use traditional FP method names at the expense of onboarding difficulty - this is an opinionated crate and I am not a purist.
### Examples
#### Inversion of control
// An effect that requires access to the inaccessible application state in the parent struct.
```rust
use effect_light::Effect;

#[derive(Default)]
struct AppState {
    count: usize,
    component: component::Component,
}
mod component {
    #[derive(Default)]
    pub struct Component {
        pub message: String
    }
    impl Component {
        pub fn handle_message(&mut self, msg: String) -> impl effect_light::Effect<&mut super::AppState> + 'static {
            self.message = msg;
            effect_light::fn_effect(|app_state: &mut super::AppState| app_state.count +=1)
        }
    }
}
let mut state = AppState::default();
let effect = state.component.handle_message("Hello, world!".to_string());
assert_eq!(state.component.message, "Hello, world!");
assert_eq!(state.count, 0);
effect.resolve(&mut state);
assert_eq!(state.count, 1);
```
#### Dependency injection
// Define generic effects, and resolve them using any type that meets the trait bounds.
```rust
use effect_light::Effect;

trait Foo {}
struct ProdFoo;
struct TestFoo;
impl Foo for ProdFoo {}
impl Foo for TestFoo {}

struct FooEffect;
impl<F: Foo> Effect<&F> for FooEffect {
    type Output = ();
    fn resolve(self, dependency: &F) {}
}

let (prod, test) = (ProdFoo, TestFoo);
// FooEffect can be resolved against both the test and prod 'foo'.
FooEffect.resolve(&prod);
FooEffect.resolve(&test);
```
#### Centralise side effects
```rust
use effect_light::Effect;

struct StdoutPrinter;
impl StdoutPrinter {
    fn print(&self, text: &str) {
        println!("{text}");
    }
}
let printer = StdoutPrinter;
let effect = effect_light::fn_effect(|printer: &StdoutPrinter| printer.print("Hello, world"));
// Prints to console here.
effect.resolve(&StdoutPrinter);
```
#### Dependency injection - async
// An effect that requires access to a `reqwest::Client`.
```rust
use effect_light::Effect;

let client = reqwest::Client::new();
let effect = effect_light::fn_effect_async(async |client: &reqwest::Client|
    client.get("https://www.google.com").send().await.unwrap().text().await.unwrap()
);
let future = effect.resolve(&client);
let rt = tokio::runtime::Runtime::new().unwrap();
let output = rt.block_on(future);
println!("{output}");
```
#### Module-level access control
// An effect that requires access to an inaccessible application state
```rust
use effect_light::{Effect, EffectExt};

#[derive(Default)]
struct AppState {
    component: component::Component,
}
mod component {
    // Note no public setters, so parent modules can't change content...
    #[derive(Default)]
    pub struct Component {
        content: String
    }
    impl Component {
        pub fn update_content<'a>() -> impl effect_light::Effect<(&'a mut Self, &'a reqwest::Client), Output=impl Future<Output = ()>>  {
            // ...but we can provide parent module an Effect that grants the required access.
            effect_light::fn_effect_async(|(this, client): (&mut Self, &reqwest::Client)| async move {
                let output = client.get("https://www.google.com").send().await.unwrap().text().await.unwrap();
                this.content = output;
            })
        }
        pub fn get_content(&self) -> &str {
            &self.content
        }
    }
}
let mut state = AppState::default();
let client = reqwest::Client::new();
let effect = component::Component::update_content().map_dependency(|(this, client): (&mut AppState, _)| (&mut this.component, client));
let future = effect.resolve((&mut state, &client));
let rt = tokio::runtime::Runtime::new().unwrap();
rt.block_on(future);
// content should contain the body of Google.com
println!("{}", state.component.get_content());
```

License: MIT
