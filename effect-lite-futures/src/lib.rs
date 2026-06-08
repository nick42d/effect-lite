#![feature(impl_trait_in_assoc_type)]
use effect_lite::{Effect, EffectExt};

pub trait EffectAsync<D>: Effect<D, Output: Future<Output = Self::FutureOutput>> {
    type FutureOutput;
}
impl<T, D, O> EffectAsync<D> for T
where
    T: crate::Effect<D>,
    T::Output: Future<Output = O>,
{
    type FutureOutput = O;
}
impl<D, T> EffectAsyncExt<D> for T where T: EffectAsync<D> {}
pub trait EffectAsyncExt<D>: EffectAsync<D> {
    fn map_async_output_async<F, Fut, T>(self, map_fn: F) -> MapAsyncOutputAsync<Self, F>
    where
        Self: Sized,
        F: Fn(Self::FutureOutput) -> Fut,
        Fut: Future<Output = T>,
    {
        MapAsyncOutputAsync {
            effect: self,
            map_fn,
        }
    }
    fn map_async_output<F, Fut, T>(self, map_fn: F) -> MapAsyncOutput<Self, F>
    where
        Self: Sized,
        F: Fn(Self::FutureOutput) -> T,
    {
        MapAsyncOutput {
            effect: self,
            map_fn,
        }
    }
    fn into_stream(self) -> IntoStream<Self>
    where
        Self: Sized,
    {
        IntoStream(self)
    }
    /// # Example
    /// ```
    /// use effect_lite::Effect;
    /// use effect_lite_futures::EffectAsyncExt;
    ///
    /// let effect_a = effect_lite::fn_effect_async(|a: String| async { a });
    /// let effect_b = effect_lite::fn_effect(|a: String| format!("{a}{a}"));
    /// let combined = effect_a.async_then(effect_b);
    /// let string = format!("Hello");
    /// assert_eq!(futures::executor::block_on(combined.resolve(string)), format!("HelloHello"));
    /// ```
    fn async_then<E>(self, next_effect: E) -> AsyncThen<Self, E>
    where
        Self: Sized,
        E: Effect<Self::FutureOutput>,
    {
        AsyncThen {
            first_effect: self,
            next_effect,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct IntoStream<E>(E);
impl<D, E> Effect<D> for IntoStream<E>
where
    E: EffectAsync<D>,
{
    type Output = futures::stream::Once<E::Output>;
    fn resolve(self, dependency: D) -> Self::Output {
        let Self(effect) = self;
        futures::stream::once(effect.resolve(dependency))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct MapAsyncOutputAsync<E, F> {
    effect: E,
    map_fn: F,
}
impl<D, E, F, Fut, T> Effect<D> for MapAsyncOutputAsync<E, F>
where
    E: EffectAsync<D>,
    F: Fn(E::FutureOutput) -> Fut,
    Fut: Future<Output = T>,
{
    type Output = futures::future::Map<E::Output, F>;
    fn resolve(self, dependency: D) -> Self::Output {
        let Self { effect, map_fn } = self;
        futures::FutureExt::map(effect.resolve(dependency), map_fn)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct MapAsyncOutput<E, F> {
    effect: E,
    map_fn: F,
}
impl<D, E, F, T> Effect<D> for MapAsyncOutput<E, F>
where
    E: EffectAsync<D>,
    F: Fn(E::FutureOutput) -> T,
{
    type Output = futures::future::Map<E::Output, F>;
    fn resolve(self, dependency: D) -> Self::Output {
        let Self { effect, map_fn } = self;
        futures::FutureExt::map(effect.resolve(dependency), map_fn)
    }
}

/// Map the output of an Effect.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct AsyncThen<E1, E2> {
    first_effect: E1,
    next_effect: E2,
}

impl<E1, E2, D> Effect<D> for AsyncThen<E1, E2>
where
    E1: EffectAsync<D>,
    E2: Effect<E1::FutureOutput>,
{
    // impl Future is used here rather than futures::future::Map becuase the closure type is unnamable.
    type Output = impl Future<Output = E2::Output>;
    fn resolve(self, dependency: D) -> Self::Output {
        let Self {
            first_effect,
            next_effect,
        } = self;
        futures::FutureExt::map(first_effect.resolve(dependency), |output| {
            next_effect.resolve(output)
        })
    }
}

pub trait EffectStream<D>: Effect<D, Output = Self::OutputStream> {
    type OutputStream: futures::Stream<Item = Self::OutputItem>;
    type OutputItem;
}
impl<T, D, I> EffectStream<D> for T
where
    T: crate::Effect<D>,
    T::Output: futures::Stream<Item = I>,
{
    type OutputStream = T::Output;
    type OutputItem = I;
}
impl<D, T> EffectStreamExt<D> for T where T: EffectStream<D> {}
pub trait EffectStreamExt<D>: EffectStream<D> {
    fn map_stream_item_async<F, Fut, T>(self, map_fn: F) -> MapStreamItemAsync<Self, F>
    where
        Self: Sized,
        F: Fn(Self::OutputItem) -> Fut,
        Fut: Future<Output = T>,
    {
        MapStreamItemAsync {
            effect: self,
            map_fn,
        }
    }
    fn map_stream_item<F, Fut, T>(self, map_fn: F) -> MapStreamItem<Self, F>
    where
        Self: Sized,
        F: Fn(Self::OutputItem) -> T,
    {
        MapStreamItem {
            effect: self,
            map_fn,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct MapStreamItemAsync<E, F> {
    effect: E,
    map_fn: F,
}
impl<D, E, F, Fut, T> Effect<D> for MapStreamItemAsync<E, F>
where
    E: EffectStream<D>,
    F: Fn(E::OutputItem) -> Fut,
    Fut: Future<Output = T>,
{
    type Output = futures::stream::Then<E::Output, Fut, F>;
    fn resolve(self, dependency: D) -> Self::Output {
        let Self { effect, map_fn } = self;
        futures::StreamExt::then(effect.resolve(dependency), map_fn)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct MapStreamItem<E, F> {
    effect: E,
    map_fn: F,
}
impl<D, E, F, T> Effect<D> for MapStreamItem<E, F>
where
    E: EffectStream<D>,
    F: Fn(E::OutputItem) -> T,
{
    type Output = futures::stream::Map<E::Output, F>;
    fn resolve(self, dependency: D) -> Self::Output {
        let Self { effect, map_fn } = self;
        futures::StreamExt::map(effect.resolve(dependency), map_fn)
    }
}
