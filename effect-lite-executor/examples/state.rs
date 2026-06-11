//! Example of an application with locally and globally scoped effects
#![feature(impl_trait_in_assoc_type)]
use crossterm::event::{Event, KeyCode, KeyEvent};
use effect_lite::{Effect, EffectExt};
use effect_lite_executor::constrained::Constraint;
use effect_lite_futures::EffectAsyncExt;
use futures::{Stream, StreamExt};
use std::{
    assert_matches,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::io::AsyncBufReadExt;

use crate::subcomponent::SubComponentStateEffect;

struct App {
    state: AppState,
    exiting: bool,
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
    Subcomponent0Effect(subcomponent::SubComponentStateEffect),
    Subcomponent1Effect(subcomponent::SubComponentStateEffect),
}

impl effect_lite::Effect<&mut AppState> for AppStateEffect {
    type Output = Option<
        impl effect_lite_futures::EffectAsync<MockNetworkServer, FutureOutput = AppStateEffect> + use<>,
    >;
    fn resolve(self, dependency: &mut AppState) -> Self::Output {
        match self {
            AppStateEffect::ChangeSubcomponent(x) => {
                assert_matches!(x, 0..=1);
                dependency.selected_component = x;
                None
            }
            AppStateEffect::Subcomponent0Effect(sub_component_state_effect) => {
                let out = sub_component_state_effect
                    .resolve(&mut dependency.subcomponent_0)
                    .map(|ef| {
                        ef.map_async_output(|out| AppStateEffect::Subcomponent0Effect(out))
                            .to_left()
                    });
                out
            }
            AppStateEffect::Subcomponent1Effect(sub_component_state_effect) => {
                let out = sub_component_state_effect
                    .resolve(&mut dependency.subcomponent_0)
                    .map(|ef| {
                        ef.map_async_output(|out| AppStateEffect::Subcomponent0Effect(out))
                            .to_right()
                    });
                out
            }
        }
    }
}

mod subcomponent {
    use effect_lite_futures::EffectAsyncExt;

    use crate::{MockNetworkServer, MockNetworkServerEffect};

    #[derive(Debug, Default)]
    pub struct SubComponentState {
        subcomponent_text: String,
        subcomponent_counter: usize,
    }
    pub struct SubComponentStateEffect {
        pub inner: SubComponentStateEffectInner,
    }
    /// Hidden inner so that SubComponentStateEffect cannot be constructed outside this module.
    pub enum SubComponentStateEffectInner {
        ReplaceText(String),
        GetAnimalAndReplaceText,
    }
    impl effect_lite::Effect<&mut SubComponentState> for SubComponentStateEffect {
        type Output = Option<
            impl effect_lite_futures::EffectAsync<
                    MockNetworkServer,
                    FutureOutput = SubComponentStateEffect,
                > + use<>,
        >;
        fn resolve(self, dependency: &mut SubComponentState) -> Self::Output {
            match self.inner {
                SubComponentStateEffectInner::ReplaceText(s) => {
                    dependency.subcomponent_counter += 1;
                    dependency.subcomponent_text = s;
                    None
                }
                SubComponentStateEffectInner::GetAnimalAndReplaceText => Some(
                    MockNetworkServerEffect::GetAnimal.map_async_output(|animal| {
                        SubComponentStateEffect {
                            inner: SubComponentStateEffectInner::ReplaceText(animal),
                        }
                    }),
                ),
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
impl effect_lite::Effect<MockNetworkServer> for MockNetworkServerEffect {
    type Output = impl Future<Output = String>;
    fn resolve(self, dependency: MockNetworkServer) -> Self::Output {
        match self {
            MockNetworkServerEffect::GetAnimal => dependency.get_animal(),
        }
    }
}
#[derive(Clone)]
pub struct MockNetworkServer {
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
    let mut app = App {
        exiting: false,
        state: Default::default(),
    };
    let server = MockNetworkServer {
        idx: Arc::new(Mutex::new(0)),
    };
    let mut executor = effect_lite_executor::Executor::new(server, allocator_api2::alloc::System);

    loop {
        if app.exiting {
            println!("Exiting!\r");
            break;
        }

        let mut event = reader.next();

        tokio::select! {
            Some(event) = event => {
                let effect = map_event(event);
                let Some(out) = effect.resolve(&mut app) else {
                    continue;
                };
                let Some(out) = out.resolve(&mut app.state) else {
                    continue;
                };
                executor.push_clone(out.into_stream_effect(), allocator_api2::alloc::System)
            }
            Some(output) = executor.get_next() => {
                match output {
                    effect_lite_executor::EffectOutput::Finished { task_id, task_type_name, task_type_id } => {
                        println!("Task finished! ID: {}, Type: {}, TypeID: {:?}\r", task_id, task_type_name, task_type_id);
                    },
                    effect_lite_executor::EffectOutput::Continuing { task_id, task_type_name, task_type_id, output_n, output } => {
                        println!("Task continuing! Output item number: {}, ID: {}, Type: {}, TypeID: {:?}\r", output_n, task_id, task_type_name, task_type_id);
                        let Some(out) = output.resolve(&mut app.state) else {
                            continue;
                        };
                        executor.push_clone(out.into_stream_effect(), allocator_api2::alloc::System)
                    },
                }
            }
        };
        println!("State is {:?}\n", app.state);
    }

    crossterm::terminal::disable_raw_mode().unwrap();
}

enum AppEffect {
    CPressed,
    APressed,
    ZeroPressed,
    OnePressed,
    EscPressed,
    UnknownEvent(crossterm::event::Event),
    Error(std::io::Error),
}

impl Effect<&mut App> for AppEffect {
    type Output = Option<AppStateEffect>;
    fn resolve(self, dependency: &mut App) -> Self::Output {
        match self {
            AppEffect::CPressed => println!("'c' pressed\r"),
            AppEffect::APressed => {
                println!("'a' pressed\r");
                match dependency.state.selected_component {
                    0 => return Some(AppStateEffect::Subcomponent0Effect(SubComponentStateEffect { inner: crate::subcomponent::SubComponentStateEffectInner::GetAnimalAndReplaceText })),
                    1 => return Some(AppStateEffect::Subcomponent1Effect(SubComponentStateEffect { inner: crate::subcomponent::SubComponentStateEffectInner::GetAnimalAndReplaceText })),
                    _ => panic!("Invalid subcomponent"),
                }
            }
            AppEffect::ZeroPressed => {
                println!("'0' pressed\r");
                return Some(AppStateEffect::ChangeSubcomponent(0));
            }
            AppEffect::OnePressed => {
                println!("'1' pressed\r");
                return Some(AppStateEffect::ChangeSubcomponent(1));
            }
            AppEffect::EscPressed => dependency.exiting = true,
            AppEffect::UnknownEvent(event) => println!("Unhandled event: {event:?}\r"),
            AppEffect::Error(e) => println!("Error: {e:?}\r"),
        };
        None
    }
}

fn map_event(event: Result<crossterm::event::Event, std::io::Error>) -> AppEffect {
    match event {
        Ok(Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            ..
        })) => AppEffect::CPressed,
        Ok(Event::Key(KeyEvent {
            code: KeyCode::Char('a'),
            ..
        })) => AppEffect::APressed,
        Ok(Event::Key(KeyEvent {
            code: KeyCode::Char('0'),
            ..
        })) => AppEffect::ZeroPressed,
        Ok(Event::Key(KeyEvent {
            code: KeyCode::Char('1'),
            ..
        })) => AppEffect::OnePressed,
        Ok(Event::Key(KeyEvent {
            code: KeyCode::Esc, ..
        })) => AppEffect::EscPressed,
        Ok(event) => AppEffect::UnknownEvent(event),
        Err(e) => AppEffect::Error(e),
    }
}
