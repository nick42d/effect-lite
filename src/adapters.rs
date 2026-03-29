use crate::Effect;
use futures::{future::Either, stream::select, Stream, StreamExt};

pub trait EffectExt<D>: Effect<D> {
    /// Map the output of an effect.
    /// ```
    /// let x = Once("Hello, world!");
    /// let y = MapOutput {
    ///     effect: x,
    ///     map_fn: |s| s.to_ascii_lowercase()
    /// };
    /// assert_eq!(y.into_stream(()).next().await, "hello, world!");
    /// ```
    fn map_output<T, F>(self, map_fn: F) -> MapOutput<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Output) -> T,
    {
        MapOutput {
            effect: self,
            map_fn,
        }
    }
    /// Map the output of an effect (async closure support)
    /// ```
    /// let x = Once("Hello, world!");
    /// let y = MapOutput {
    ///     effect: x,
    ///     map_fn: |s| s.to_ascii_lowercase()
    /// };
    /// assert_eq!(y.into_stream(()).next().await, "hello, world!");
    /// ```
    fn map_output_async<T, F, Fut>(self, map_fn: F) -> MapOutput<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Output) -> Fut,
        Fut: Future<Output = T>,
    {
        MapOutput {
            effect: self,
            map_fn,
        }
    }
    fn map_dependency<D2, F>(self, map_fn: F) -> MapDependency<Self, F>
    where
        Self: Sized,
        F: FnOnce(D2) -> D,
    {
        MapDependency {
            effect: self,
            map_fn,
        }
    }
    /// Merge two effects.
    /// # Resolution note
    /// The first effect is resolved first.
    // TODO: how to swap resolution order...
    fn merge<E2, D2>(self, other: E2) -> Merge<Self, E2>
    where
        Self: Sized,
        E2: Effect<D2>,
    {
        Merge {
            effect_1: self,
            effect_2: other,
        }
    }
}

/// Map the output of an Effect.
pub struct MapOutput<E, F> {
    effect: E,
    map_fn: F,
}

impl<D, E, F, T> Effect<D> for MapOutput<E, F>
where
    E: Effect<D>,
    F: Fn(E::Output) -> T,
{
    type Output = T;
    fn resolve(self, dependency: D) -> Self::Output {
        let Self { effect, map_fn } = self;
        map_fn(effect.resolve(dependency))
    }
}

pub struct MapDependency<E, F> {
    effect: E,
    map_fn: F,
}

impl<D1, D2, E, F> Effect<D2> for MapDependency<E, F>
where
    E: Effect<D1>,
    F: FnOnce(D2) -> D1,
{
    type Output = E::Output;
    fn resolve(self, dependency: D2) -> Self::Output {
        let Self { effect, map_fn } = self;
        effect.resolve(map_fn(dependency))
    }
}

pub struct Merge<E1, E2> {
    effect_1: E1,
    effect_2: E2,
}

impl<D1, D2, E1, E2> Effect<(D1, D2)> for Merge<E1, E2>
where
    E1: Effect<D1>,
    E2: Effect<D2>,
{
    type Output = (E1::Output, E2::Output);
    fn resolve(self, dependency: (D1, D2)) -> Self::Output {
        let Self { effect_1, effect_2 } = self;
        let (d1, d2) = dependency;
        (effect_1.resolve(d1), effect_2.resolve(d2))
    }
}
