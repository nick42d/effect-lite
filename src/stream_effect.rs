use crate::Effect;

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
#[cfg(feature = "futures")]
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
#[cfg(feature = "futures")]
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
