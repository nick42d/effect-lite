use crate::Effect;
use futures::{future::Either, stream::select, Stream, StreamExt};

pub trait EffectExt<D>: Effect<D> {
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
    /// Map the output and dependency of an Effect.
    /// ```
    /// let x = "Hello, world";
    /// let y = once_dependency(&x).map_output(|_| "world");
    /// let z = y.map(|dependency: &str, output: &str| dependency.find(frontend);
    /// assert_eq!(y.into_stream(()).next().await, "hello, world!");
    /// ```
    fn map<T, F>(self, map_fn: F) -> Map<Self, F>
    where
        Self: Sized,
        F: FnOnce(D, Self::Output) -> T,
    {
        Map {
            effect: self,
            map_fn,
        }
    }
    fn select_either<E2, D2>(self, other: E2) -> SelectEither<Self, E2>
    where
        Self: Sized,
        E2: Effect<D2>,
    {
        SelectEither {
            effect_1: self,
            effect_2: other,
        }
    }
}
pub struct Map<E, F> {
    effect: E,
    map_fn: F,
}

// impl<D, E, F, T> Effect<D> for Map<E, F>
// where
//     E: Effect<D>,
//     F: FnMut(D, E::Output) -> T,
// {
//     type Output = T;
//     // TODO
//     fn into_stream(self, dependency: D) -> impl Stream<Item = Self::Output> {
//         let Self { effect, map_fn } = self;
//         effect.into_stream(dependency).map(map_fn)
//     }
// }

/// Map the output of an Effect.
/// ```
/// let x = Once("Hello, world!");
/// let y = MapOutput {
///     effect: x,
///     map_fn: |s| s.to_ascii_lowercase()
/// };
/// assert_eq!(y.into_stream(()).next().await, "hello, world!");
/// ```
pub struct MapOutput<E, F> {
    effect: E,
    map_fn: F,
}

impl<D, E, F, T> Effect<D> for MapOutput<E, F>
where
    E: Effect<D>,
    F: FnMut(E::Output) -> T,
{
    type Output = T;
    fn into_stream(self, dependency: D) -> impl Stream<Item = Self::Output> {
        let Self { effect, map_fn } = self;
        effect.into_stream(dependency).map(map_fn)
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
    fn into_stream(self, dependency: D2) -> impl Stream<Item = Self::Output> {
        let Self { effect, map_fn } = self;
        effect.into_stream(map_fn(dependency))
    }
}

pub struct SelectEither<E1, E2> {
    effect_1: E1,
    effect_2: E2,
}

impl<D1, D2, E1, E2> Effect<(D1, D2)> for SelectEither<E1, E2>
where
    E1: Effect<D1>,
    E2: Effect<D2>,
{
    type Output = Either<E1::Output, E2::Output>;
    fn into_stream(self, dependency: (D1, D2)) -> impl Stream<Item = Self::Output> {
        let Self { effect_1, effect_2 } = self;
        let (d1, d2) = dependency;
        select(
            effect_1.into_stream(d1).map(Either::Left),
            effect_2.into_stream(d2).map(Either::Right),
        )
    }
}
