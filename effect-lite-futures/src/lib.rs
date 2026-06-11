use effect_lite::Effect;
use pin_project::pin_project;

pub trait IntoEffectAsync<D> {
    type FutureOutput;
    fn into_effect_async(self)
        -> impl Effect<D, Output = impl Future<Output = Self::FutureOutput>>;
}
impl<T, D, O> IntoEffectAsync<D> for T
where
    T: crate::Effect<D>,
    T::Output: Future<Output = O>,
{
    type FutureOutput = O;
    fn into_effect_async(
        self,
    ) -> impl Effect<D, Output = impl Future<Output = Self::FutureOutput>> {
        self
    }
}

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
    fn map_async_output<F, T>(self, map_fn: F) -> MapAsyncOutput<Self, F>
    where
        Self: Sized,
        F: Fn(Self::FutureOutput) -> T,
    {
        MapAsyncOutput {
            effect: self,
            map_fn,
        }
    }
    fn into_stream_effect(self) -> IntoStreamEffect<Self>
    where
        Self: Sized,
    {
        IntoStreamEffect(self)
    }
    fn to_left_async<R>(self) -> AsyncEither<Self, R>
    where
        Self: Sized,
        R: EffectAsync<D, FutureOutput = Self::FutureOutput>,
    {
        AsyncEither::Left(self)
    }
    fn to_right_async<L>(self) -> AsyncEither<L, Self>
    where
        Self: Sized,
        L: EffectAsync<D, FutureOutput = Self::FutureOutput>,
    {
        AsyncEither::Right(self)
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
pub struct IntoStreamEffect<E>(E);
impl<D, E> Effect<D> for IntoStreamEffect<E>
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum AsyncEither<L, R> {
    Left(L),
    Right(R),
}

impl<L, R, D> Effect<D> for AsyncEither<L, R>
where
    L: EffectAsync<D>,
    R: EffectAsync<D, FutureOutput = L::FutureOutput>,
{
    type Output = futures::future::Either<L::Output, R::Output>;
    fn resolve(self, dependency: D) -> Self::Output {
        match self {
            AsyncEither::Left(l) => futures::future::Either::Left(l.resolve(dependency)),
            AsyncEither::Right(r) => futures::future::Either::Right(r.resolve(dependency)),
        }
    }
}

/// Map the output of an Effect.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct AsyncThen<E1, E2> {
    first_effect: E1,
    next_effect: E2,
}

#[pin_project(project = AsyncThenProj, project_replace = AsyncThenProjReplace)]
// #[project_replace = MapProjReplace]
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Debug)]
pub enum AsyncThenOutput<F, E> {
    Incomplete {
        #[pin]
        future: F,
        next_effect: E,
    },
    Complete,
}

impl<F, E> futures::future::FusedFuture for AsyncThenOutput<F, E>
where
    F: Future,
    E: Effect<F::Output>,
{
    fn is_terminated(&self) -> bool {
        todo!()
    }
}

impl<F, E> Future for AsyncThenOutput<F, E>
where
    F: Future,
    E: Effect<F::Output>,
{
    type Output = E::Output;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        // Implementation to match futures::future::Map
        match self.as_mut().project() {
            AsyncThenProj::Incomplete { future, .. } => {
                let output = futures::ready!(future.poll(cx));
                match self.project_replace(Self::Complete) {
                    AsyncThenProjReplace::Incomplete { next_effect, .. } => {
                        std::task::Poll::Ready(next_effect.resolve(output))
                    }
                    AsyncThenProjReplace::Complete => unreachable!(),
                }
            }
            AsyncThenProj::Complete => {
                panic!("AsyncThenOutput must not be polled after it returned 'Poll::Ready'")
            }
        }
    }
}

impl<E1, E2, D> Effect<D> for AsyncThen<E1, E2>
where
    E1: EffectAsync<D>,
    E2: Effect<E1::FutureOutput>,
{
    type Output = AsyncThenOutput<E1::Output, E2>;
    fn resolve(self, dependency: D) -> Self::Output {
        let Self {
            first_effect,
            next_effect,
        } = self;
        AsyncThenOutput::Incomplete {
            future: first_effect.resolve(dependency),
            next_effect,
        }
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
    /// # Example
    /// ```
    /// use effect_lite::Effect;
    /// use effect_lite_futures::EffectAsyncExt;
    /// use effect_lite_futures::EffectStreamExt;
    ///
    /// let effect_a = effect_lite::fn_effect_async(|a: String| async { a }).into_stream_effect();
    /// let effect_b = effect_lite::fn_effect(|a: String| format!("{a}{a}"));
    /// let combined = effect_a.stream_then(effect_b);
    /// let string = format!("Hello");
    /// let combined_stream = combined.resolve(string);
    /// let combined_stream_pinned = std::pin::pin!(combined_stream);
    /// assert_eq!(futures::executor::block_on_stream(combined_stream_pinned).collect::<Vec<_>>(), vec![format!("HelloHello")]);
    /// ```
    fn stream_then<E>(self, next_effect: E) -> StreamThen<Self, E>
    where
        Self: Sized,
        E: Effect<Self::OutputItem> + Clone,
    {
        StreamThen {
            first_effect: self,
            next_effect,
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
/// Map the output of an Effect.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct StreamThen<E1, E2> {
    first_effect: E1,
    next_effect: E2,
}

impl<E1, E2, D> Effect<D> for StreamThen<E1, E2>
where
    E1: EffectStream<D>,
    E2: Effect<E1::OutputItem>,
{
    type Output = StreamThenOutput<E1::Output, E2>;
    fn resolve(self, dependency: D) -> Self::Output {
        let Self {
            first_effect,
            next_effect,
        } = self;
        StreamThenOutput::Incomplete {
            future: first_effect.resolve(dependency),
            next_effect,
        }
    }
}

#[pin_project(project = StreamThenProj, project_replace = StreamThenReplace)]
// #[project_replace = MapProjReplace]
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Debug)]
pub enum StreamThenOutput<F, E> {
    Incomplete {
        #[pin]
        future: F,
        next_effect: E,
    },
    Complete,
}

impl<F, E> futures::Stream for StreamThenOutput<F, E>
where
    F: futures::Stream,
    E: Effect<F::Item> + Clone,
{
    type Item = E::Output;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}
