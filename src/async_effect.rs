use crate::Effect;

pub trait EffectAsync<D>: Effect<D, Output = Self::OutputFut> {
    type OutputFut: Future<Output = Self::OutputAsync>;
    type OutputAsync;
}
impl<T, D, O> EffectAsync<D> for T
where
    T: crate::Effect<D>,
    T::Output: Future<Output = O>,
{
    type OutputFut = T::Output;
    type OutputAsync = O;
}
impl<D, T> EffectAsyncExt<D> for T where T: EffectAsync<D> {}
pub trait EffectAsyncExt<D>: EffectAsync<D> {
    #[cfg(feature = "futures")]
    fn map_async_output_async<F, Fut, T>(self, map_fn: F) -> MapAsyncOutputAsync<Self, F>
    where
        Self: Sized,
        F: Fn(Self::OutputAsync) -> Fut,
        Fut: Future<Output = T>,
    {
        MapAsyncOutputAsync {
            effect: self,
            map_fn,
        }
    }
    #[cfg(feature = "futures")]
    fn map_async_output<F, Fut, T>(self, map_fn: F) -> MapAsyncOutput<Self, F>
    where
        Self: Sized,
        F: Fn(Self::OutputAsync) -> T,
    {
        MapAsyncOutput {
            effect: self,
            map_fn,
        }
    }
}
#[cfg(feature = "futures")]
pub struct MapAsyncOutputAsync<E, F> {
    effect: E,
    map_fn: F,
}
#[cfg(feature = "futures")]
impl<D, E, F, Fut, T> Effect<D> for MapAsyncOutputAsync<E, F>
where
    E: EffectAsync<D>,
    F: Fn(E::OutputAsync) -> Fut,
    Fut: Future<Output = T>,
{
    type Output = futures::future::Map<E::Output, F>;
    fn resolve(self, dependency: D) -> Self::Output {
        let Self { effect, map_fn } = self;
        futures::FutureExt::map(effect.resolve(dependency), map_fn)
    }
}
#[cfg(feature = "futures")]
pub struct MapAsyncOutput<E, F> {
    effect: E,
    map_fn: F,
}
#[cfg(feature = "futures")]
impl<D, E, F, T> Effect<D> for MapAsyncOutput<E, F>
where
    E: EffectAsync<D>,
    F: Fn(E::OutputAsync) -> T,
{
    type Output = futures::future::Map<E::Output, F>;
    fn resolve(self, dependency: D) -> Self::Output {
        let Self { effect, map_fn } = self;
        futures::FutureExt::map(effect.resolve(dependency), map_fn)
    }
}
