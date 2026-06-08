//! Example of an application with locally and globally scoped effects
#![feature(impl_trait_in_assoc_type)]
use crossterm::event::{Event, KeyCode, KeyEvent};
use effect_light::Effect;
use effect_lite_executor::constrained::Constraint;
use futures::{Stream, StreamExt};
use std::{
    assert_matches,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::io::AsyncBufReadExt;

struct App {
    state: AppState,
}

#[derive(Debug, Default)]
struct AppState {
    app_text: String,
    app_counter: usize,
    selected_component: usize,
    subcomponent_0: subcomponent::SubComponentState,
    subcomponent_1: subcomponent::SubComponentState,
}

enum AppStateEffect {
    ChangeSubcomponent(usize),
    Subcomponent0Effect(SubComponentStateEffect),
    Subcomponent1Effect(SubComponentStateEffect),
}

impl effect_light::Effect<&mut AppState> for AppStateEffect {
    type Output = ();

    fn resolve(self, dependency: &mut AppState) -> Self::Output {
        match self {
            AppStateEffect::ChangeSubcomponent(x) => {
                assert_matches!(x, 0..=1);
                dependency.selected_component = x;
            }
        }
    }
}

mod subcomponent {
    use crate::{MockNetworkServer, MockNetworkServerEffect};

    #[derive(Debug, Default)]
    pub struct SubComponentState {
        subcomponent_text: String,
        subcomponent_counter: usize,
    }
    enum SubComponentStateEffect {
        ReplaceText(String),
        GetAnimalAndReplaceText,
    }
    impl effect_light::Effect<&mut SubComponentState> for SubComponentStateEffect {
        type Output =
            Option<impl effect_light::Effect<MockNetworkServer, Output = SubComponentStateEffect>>;
        fn resolve(self, dependency: &mut SubComponentState) -> Self::Output {
            match self {
                SubComponentStateEffect::ReplaceText(s) => {
                    dependency.subcomponent_counter += 1;
                    dependency.subcomponent_text = s;
                    None
                }
                SubComponentStateEffect::GetAnimalAndReplaceText => {
                    MockNetworkServerEffect::GetAnimal;
                }
            }
        }
    }
}

struct StdoutPrinter;
impl StdoutPrinter {
    fn println(s: impl std::fmt::Display) {
        println!("{s}");
    }
}
enum MockNetworkServerEffect {
    GetAnimal,
}
impl effect_light::Effect<MockNetworkServer> for MockNetworkServerEffect {
    type Output = impl Future<Output = String>;
    fn resolve(self, dependency: MockNetworkServer) -> Self::Output {
        match self {
            MockNetworkServerEffect::GetAnimal => dependency.get_animal(),
        }
    }
}
#[derive(Clone)]
struct MockNetworkServer {
    idx: Arc<Mutex<usize>>,
}
impl MockNetworkServer {
    async fn get_animal(self) -> String {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let words = ["Dog", "Cat", "Rabbit", "CoW", "Pigeon"];
        let mut guard = self.idx.lock().unwrap();
        let idx = *guard;
        *guard += 1;
        *guard %= 5;
        drop(guard);
        words[idx].to_string()
    }
}

#[tokio::main]
async fn main() {
    crossterm::terminal::enable_raw_mode().unwrap();

    println!("'a' to get next animal and apply it to subcomponent, 'Esc' to quit, '0' to select component 0, '1' to select component '1'\r");
    let mut reader = crossterm::event::EventStream::new();
    let mut state = AppState::default();

    loop {
        let mut event = reader.next();

        tokio::select! {
            maybe_event = event => {
                match maybe_event {
                    Some(Ok(Event::Key(KeyEvent {code: KeyCode::Char('c'), .. } ))) => {
                        println!("'c' pressed\r");
                    }
                    Some(Ok(Event::Key(KeyEvent {code: KeyCode::Char('a'), .. } ))) => {
                        println!("'a' pressed\r");
                    }
                    Some(Ok(Event::Key(KeyEvent {code: KeyCode::Char('0'), .. } ))) => {
                       println!("'0' pressed\r");
                       AppStateEffect::ChangeSubcomponent(0).resolve(&mut state);
                    }
                    Some(Ok(Event::Key(KeyEvent {code: KeyCode::Char('1'), .. } ))) => {
                        println!("'1' pressed\r");
                       AppStateEffect::ChangeSubcomponent(1).resolve(&mut state);
                    }
                    Some(Ok(Event::Key(KeyEvent {code: KeyCode::Esc, .. } ))) => break,
                    Some(Ok(event)) => println!("Unhandled event: {event:?}\r"),
                    Some(Err(e)) => println!("Error: {e:?}\r"),
                    None => break,
                }
                println!("State: {state:?}\r");
            }
        };
    }

    crossterm::terminal::disable_raw_mode().unwrap();
}
