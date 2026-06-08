//! A concurrent (not parrallel) round-robin executor for [effect_lite::Effect]s.
//! The executor is no-std, however an allocator is required.
//! # Usage example
//! ```
//! use effect_lite::EffectExt;
//!
//! static ALLOC: static_cell::StaticCell<stalloc::Stalloc<1000,8>> = static_cell::StaticCell::new();
//! let alloc_ref: &'static mut stalloc::Stalloc<1000,8> = ALLOC.init(stalloc::Stalloc::new());
//!
//! let mut side_effect_counter: usize = 0;
//!
//! #[derive(Clone)]
//! struct AddOne;
//!
//! impl effect_lite::Effect<&mut usize> for AddOne {
//!     type Output = usize;
//!     fn resolve(self, dependency: &mut usize) -> usize {
//!         *dependency += 1;
//!         *dependency
//!     }
//! }
//!
//! let effect = AddOne.map_output(|x| futures::stream::once(async move {x}));
//!
//! let mut executor: effect_lite_executor::Executor<_, usize, usize, &stalloc::Stalloc<_,_>> =
//!     effect_lite_executor::Executor::new(side_effect_counter, alloc_ref);
//! executor.push_mut(effect.clone(), alloc_ref);
//! executor.push_mut(effect, alloc_ref);
//!
//! assert_eq!(*executor.get_dependencies(), 2);
//!
//! let next_1 = futures::executor::block_on(executor.get_next());
//! let next_2 = futures::executor::block_on(executor.get_next());
//! let next_3 = futures::executor::block_on(executor.get_next());
//! let next_4 = futures::executor::block_on(executor.get_next());
//! let next_5 = futures::executor::block_on(executor.get_next());
//! eprintln!("{:?}", next_1.unwrap());
//! eprintln!("{:?}", next_2.unwrap());
//! eprintln!("{:?}", next_3.unwrap());
//! eprintln!("{:?}", next_4.unwrap());
//! assert!(next_5.is_none());
//!
//! ```
#![no_std]

use allocator_api2::{alloc::Allocator, boxed::Box, vec::Vec};
use core::{
    any::{Any, TypeId},
    marker::PhantomData,
    pin::Pin,
    task::Poll,
};
use effect_lite::Effect;
use futures::StreamExt;

pub mod constrained {
    use core::any::Any;

    #[derive(Eq, PartialEq, Debug)]
    pub struct Constraint<Md> {
        pub(crate) constraint_type: ConstraitType<Md>,
    }

    impl<Md> Constraint<Md> {
        pub fn new_block_same_type() -> Self {
            Self {
                constraint_type: ConstraitType::BlockSameType,
            }
        }
        pub fn new_kill_same_type() -> Self {
            Self {
                constraint_type: ConstraitType::KillSameType,
            }
        }
        pub fn new_block_matching_metadata(metadata: Md) -> Self {
            Self {
                constraint_type: ConstraitType::BlockMatchingMetatdata(metadata),
            }
        }
    }

    #[derive(Eq, PartialEq, Debug)]
    pub enum ConstraitType<Md> {
        BlockSameType,
        KillSameType,
        BlockMatchingMetatdata(Md),
    }
    struct EffectConstrained<E, Md> {
        constraint: Constraint<Md>,
        effect: E,
        metadata: allocator_api2::vec::Vec<Md>,
    }
    struct ConstrainedEffectExecutor<E, T, D> {
        inner: super::Executor<E, T, D, allocator_api2::alloc::Global>,
        info_list: allocator_api2::vec::Vec<()>,
    }
    impl<E, T, D> ConstrainedEffectExecutor<E, T, D> {
        pub fn new(dependencies: D) -> Self {
            let inner = crate::Executor::new(dependencies, allocator_api2::alloc::Global)
                .with_on_push_cb(|_| ());
            Self {
                inner,
                info_list: allocator_api2::vec![],
            }
        }
        pub fn push_ref<S, Md>(&mut self, effect: EffectConstrained<E, Md>)
        where
            E: for<'b> effect_lite::Effect<&'b D, Output = S> + Any + 'static,
            S: futures::Stream<Item = T> + Any + 'static,
            T: 'static,
        {
            let EffectConstrained {
                constraint,
                effect,
                metadata,
            } = effect;
            self.inner.push_ref(effect, allocator_api2::alloc::Global);
        }
        pub fn get_next(&mut self) -> crate::GetNext<T, allocator_api2::alloc::Global> {
            self.inner.get_next()
        }
        pub fn get_dependencies(&self) -> &D {
            &self.inner.dependencies
        }
        pub fn get_dependencies_mut(&mut self) -> &mut D {
            &mut self.inner.dependencies
        }
    }
}

type NoDepsEffect<T> = dyn futures::Stream<Item = T>;
type SmallboxSize = smallbox::space::S8;

// Note - may be able to directly hold allocator - tbc
pub struct Executor<E, T, D, A: Allocator> {
    next_task_id: u64,
    last_polled: usize,
    task_list: Vec<ExecutorItem<T, A>, A>,
    on_push_cb: Option<fn(E)>,
    // This allows the executor to assert certain things about the effect - e.g, force it to be Debug.
    effect_type: PhantomData<E>,
    dependencies: D,
}

struct Enumerated<T>(usize, T);

struct ExecutorItem<T, A: Allocator> {
    stream: Pin<Box<NoDepsEffect<Enumerated<T>>, A>>,
    task_id: u64,
    task_type_name: &'static str,
    task_type_id: TypeId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EffectOutput<T> {
    Finished {
        task_id: u64,
        task_type_name: &'static str,
        task_type_id: TypeId,
    },
    Continuing {
        task_id: u64,
        task_type_name: &'static str,
        task_type_id: TypeId,
        output_n: usize,
        output: T,
    },
}

impl<E, T, A: Allocator + 'static, D> Executor<E, T, D, A> {
    pub fn new(dependencies: D, alloc: A) -> Self {
        Self {
            next_task_id: 0,
            task_list: Vec::new_in(alloc),
            last_polled: 0,
            on_push_cb: None,
            effect_type: PhantomData,
            dependencies,
        }
    }
    pub fn with_on_push_cb(self, cb: fn(E)) -> Executor<E, T, D, A> {
        let Self {
            next_task_id,
            last_polled,
            task_list,
            on_push_cb: _,
            effect_type,
            dependencies,
        } = self;
        Executor {
            next_task_id,
            last_polled,
            task_list,
            on_push_cb: Some(cb),
            effect_type,
            dependencies,
        }
    }
    pub fn push_clone<S>(&mut self, effect: E, alloc: A)
    where
        A: 'static,
        E: Effect<D, Output = S> + Any + 'static,
        S: futures::Stream<Item = T> + Any + 'static,
        T: 'static,
        D: Clone,
    {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        let task_type_name = core::any::type_name_of_val(&effect);
        let task_type_id = effect.type_id();
        let stream = effect.resolve(self.dependencies.clone());
        let stream: Box<NoDepsEffect<Enumerated<T>>, A> =
            allocator_api2::unsize_box!(allocator_api2::boxed::Box::new_in(
                stream.enumerate().map(|(idx, item)| Enumerated(idx, item)),
                alloc,
            ));
        self.task_list.push(ExecutorItem {
            stream: Pin::from(stream),
            task_id,
            task_type_name,
            task_type_id,
        })
    }
    pub fn push_ref<S>(&mut self, effect: E, alloc: A)
    where
        A: 'static,
        E: for<'b> Effect<&'b D, Output = S> + Any + 'static,
        S: futures::Stream<Item = T> + Any + 'static,
        T: 'static,
    {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        let task_type_name = core::any::type_name_of_val(&effect);
        let task_type_id = effect.type_id();
        let stream = effect.resolve(&self.dependencies);
        let stream: Box<NoDepsEffect<Enumerated<T>>, A> =
            allocator_api2::unsize_box!(allocator_api2::boxed::Box::new_in(
                stream.enumerate().map(|(idx, item)| Enumerated(idx, item)),
                alloc,
            ));
        self.task_list.push(ExecutorItem {
            stream: Pin::from(stream),
            task_id,
            task_type_name,
            task_type_id,
        })
    }
    pub fn push_mut<S>(&mut self, effect: E, alloc: A)
    where
        A: 'static,
        E: for<'b> Effect<&'b mut D, Output = S> + Any + 'static,
        S: futures::Stream<Item = T> + Any + 'static,
        T: 'static,
    {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        let task_type_name = core::any::type_name_of_val(&effect);
        let task_type_id = effect.type_id();
        let stream = effect.resolve(&mut self.dependencies);
        let stream: Box<NoDepsEffect<Enumerated<T>>, A> =
            allocator_api2::unsize_box!(allocator_api2::boxed::Box::new_in(
                stream.enumerate().map(|(idx, item)| Enumerated(idx, item)),
                alloc,
            ));
        self.task_list.push(ExecutorItem {
            stream: Pin::from(stream),
            task_id,
            task_type_name,
            task_type_id,
        })
    }
    pub fn get_next(&mut self) -> GetNext<T, A> {
        GetNext {
            items: &mut self.task_list,
            last_polled: &mut self.last_polled,
        }
    }
    pub fn get_dependencies(&self) -> &D {
        &self.dependencies
    }
    pub fn get_dependencies_mut(&mut self) -> &mut D {
        &mut self.dependencies
    }
}

pub struct GetNext<'a, T, A: Allocator> {
    items: &'a mut Vec<ExecutorItem<T, A>, A>,
    last_polled: &'a mut usize,
}

impl<'a, T, A: Allocator> futures::Future for GetNext<'a, T, A> {
    type Output = Option<EffectOutput<T>>;
    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let len = self.items.len();
        for idx in 0..len {
            // TODO: Tests for poll order
            *self.last_polled += 1;
            let adj_idx = (idx + *self.last_polled) % len;
            let mut stream = self
                .items
                .get_mut(adj_idx)
                .expect("adj_idx should be within bounds since it's modulod with len")
                .stream
                .as_mut();
            match futures::stream::Stream::poll_next(stream.as_mut(), cx) {
                Poll::Ready(Option::None) => {
                    // NOPANIC: len checked above
                    let task_id = self.items[idx].task_id;
                    let task_type_name = self.items[idx].task_type_name;
                    let task_type_id = self.items[idx].task_type_id;
                    self.items.remove(idx);
                    return Poll::Ready(Some(EffectOutput::Finished {
                        task_id,
                        task_type_name,
                        task_type_id,
                    }));
                }
                Poll::Ready(Option::Some(Enumerated(output_n, output))) => {
                    // NOPANIC: len checked above
                    let task_id = self.items[idx].task_id;
                    let task_type_name = self.items[idx].task_type_name;
                    let task_type_id = self.items[idx].task_type_id;
                    return Poll::Ready(Some(EffectOutput::Continuing {
                        task_id,
                        task_type_name,
                        task_type_id,
                        output_n,
                        output,
                    }));
                }
                Poll::Pending => (),
            }
        }
        Poll::Ready(None)
    }
}

#[cfg(feature = "tokio")]
mod tokio;

#[cfg(test)]
mod tests {}
