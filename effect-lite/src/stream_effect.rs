use crate::Effect;

pub trait EffectStream<D>: Effect<D, Output: futures::Stream<Item = Self::StreamItem>> {
    type StreamItem;
}
impl<T, D, I> EffectStream<D> for T
where
    T: crate::Effect<D>,
    T::Output: futures::Stream<Item = I>,
{
    type StreamItem = I;
}
impl<D, T> EffectStreamExt<D> for T where T: EffectStream<D> {}
pub trait EffectStreamExt<D>: EffectStream<D> {
    fn map_stream_item_async<F, Fut, T>(self, map_fn: F) -> MapStreamItemAsync<Self, F>
    where
        Self: Sized,
        F: Fn(Self::StreamItem) -> Fut,
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
        F: Fn(Self::StreamItem) -> T,
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
#[cfg(feature = "futures")]
impl<D, E, F, Fut, T> Effect<D> for MapStreamItemAsync<E, F>
where
    E: EffectStream<D>,
    F: Fn(E::StreamItem) -> Fut,
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
#[cfg(feature = "futures")]
impl<D, E, F, T> Effect<D> for MapStreamItem<E, F>
where
    E: EffectStream<D>,
    F: Fn(E::StreamItem) -> T,
{
    type Output = futures::stream::Map<E::Output, F>;
    fn resolve(self, dependency: D) -> Self::Output {
        let Self { effect, map_fn } = self;
        futures::StreamExt::map(effect.resolve(dependency), map_fn)
    }
}
