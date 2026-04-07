use core::ops::{Deref, DerefMut};

use crate::{either::Either, Effect};

pub trait EffectExt<D>: Effect<D> {
    /// Map the output of an effect.
    /// ```
    /// use effect_light::{Effect, EffectExt};
    ///
    /// let x = effect_light::value("Hello, world!").map_output(|s: &str| s.to_ascii_lowercase());
    /// assert_eq!(x.resolve(()), "hello, world!");
    /// ```
    fn map_output<T, F>(self, map_fn: F) -> MapOutput<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Output) -> T,
    {
        MapOutput {
            effect: self,
            map_fn,
        }
    }
    /// Map the output of an effect (async closure support)
    /// ```
    /// use effect_light::{Effect, EffectExt};
    ///
    /// # futures::executor::block_on(async {
    /// let x = effect_light::value("Hello, world!").map_output_async(async |s| s.to_ascii_lowercase());
    /// assert_eq!(x.resolve(()).await, "hello, world!");
    /// # })
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
    /// Map the dependency of an effect.
    /// ```
    /// use effect_light::{Effect, EffectExt};
    ///
    /// let x = effect_light::echo::<String>().map_dependency(|s: &str| s.to_string());
    /// assert_eq!(x.resolve("Hello, world!"), String::from("Hello, world!"));
    /// ```
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
    /// ```
    /// use effect_light::{Effect, EffectExt};
    ///
    /// let x = effect_light::fn_effect(|s: &str| s.to_ascii_lowercase());
    /// let y = effect_light::fn_effect(|s: &str| s.to_ascii_uppercase());
    /// assert_eq!(x.merge(y).resolve(("Hello","world")), ("hello".to_string(),"WORLD".to_string()));
    /// ```
    /// # Resolution note
    /// The first effect is resolved first.
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
    /// Flatten an effect returning an optional effect into a single effect returning an optional
    fn flatten_option<E2, D2>(self) -> FlattenOption<Self>
    where
        Self: Sized,
        Self: Effect<D, Output = Option<E2>>,
        E2: Effect<D2>,
    {
        FlattenOption(self)
    }
    /// Flatten an effect returning an effect into a single effect
    fn flatten<E2, D2>(self) -> Flatten<Self>
    where
        Self: Sized,
        Self: Effect<D, Output = E2>,
        E2: Effect<D2>,
    {
        Flatten(self)
    }
    /// Flatten an effect returning an effect into a single effect
    fn collapse<D2>(self) -> Collapse<Self>
    where
        Self: Sized,
        Self: Effect<(D2, D2)>,
        D2: Clone,
    {
        Collapse(self)
    }
    /// Flatten an effect returning an effect (with the same dependency) into a single effect
    fn flat_collapse<E2>(self) -> FlatCollapse<Self>
    where
        Self: Sized,
        Self: Effect<D, Output = E2>,
        E2: Effect<D>,
    {
        FlatCollapse(self)
    }
    /// Provide the left dependency of an effect with a 2-tuple dependency.
    fn provide_left<D1, D2>(self, left_dependency: D1) -> ProvideLeft<Self, D1>
    where
        Self: Sized,
        Self: Effect<(D1, D2)>,
    {
        ProvideLeft {
            effect: self,
            left_dependency,
        }
    }
    /// Provide the right dependency of an effect with a 2-tuple dependency.
    fn provide_right<D1, D2>(self, right_dependency: D2) -> ProvideRight<Self, D2>
    where
        Self: Sized,
        Self: Effect<(D1, D2)>,
    {
        ProvideRight {
            effect: self,
            right_dependency,
        }
    }
    /// Helper function to wrap in the left side of a [crate::either::Either]
    /// ```
    /// use effect_light::EffectExt;
    ///
    /// fn diverges(x: bool) -> impl effect_light::Effect<()> {
    ///     match x {
    ///         true => effect_light::fn_effect(|_| "Hello").to_left(),
    ///         false => effect_light::fn_effect(|_| "World!").to_right(),
    ///     }
    /// }
    /// ```
    fn to_left<R>(self) -> Either<Self, R>
    where
        Self: Sized,
        R: Effect<D, Output = Self::Output>,
    {
        Either::Left(self)
    }
    /// Helper function to wrap in the right side of a [crate::either::Either]
    /// ```
    /// use effect_light::EffectExt;
    ///
    /// fn diverges(x: bool) -> impl effect_light::Effect<()> {
    ///     match x {
    ///         true => effect_light::fn_effect(|_| "Hello").to_left(),
    ///         false => effect_light::fn_effect(|_| "World!").to_right(),
    ///     }
    /// }
    /// ```
    fn to_right<L>(self) -> Either<L, Self>
    where
        Self: Sized,
        L: Effect<D, Output = Self::Output>,
    {
        Either::Right(self)
    }
}

pub trait EffectExt2<'a, D>: Effect<&'a mut D>
where
    D: 'a,
{
    /// Flatten an effect returning an effect (with the same dependency) into a single effect, where the dependency is a mutable reference
    fn flat_collapse_mut<E2>(self) -> FlatCollapseMut<Self>
    where
        Self: Sized,
        Self: Effect<&'a mut D, Output = E2>,
        E2: for<'b> Effect<&'b mut D>,
        <E2 as Effect<&'a mut D>>::Output: 'static,
    {
        FlatCollapseMut(self)
    }
}

impl<'a, D, T> EffectExt2<'a, D> for T
where
    T: Effect<&'a mut D>,
    D: 'a,
{
}
impl<D, T> EffectExt<D> for T where T: Effect<D> {}

/// Map the output of an Effect.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
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

/// Map the dependency of an Effect.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
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

/// Merge two effects.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
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

/// Flatten an effect returning an optional effect into a single effect returning an optional
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct FlattenOption<E1>(E1);

impl<D1, D2, E1, E2> Effect<(D1, D2)> for FlattenOption<E1>
where
    E1: Effect<D1, Output = Option<E2>>,
    E2: Effect<D2>,
{
    type Output = Option<E2::Output>;
    fn resolve(self, dependency: (D1, D2)) -> Self::Output {
        let (d1, d2) = dependency;
        self.0.resolve(d1).map(|e| e.resolve(d2))
    }
}

/// Flatten an effect returning an effect into a single effect
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Flatten<E1>(E1);

impl<D1, D2, E1, E2> Effect<(D1, D2)> for Flatten<E1>
where
    E1: Effect<D1, Output = E2>,
    E2: Effect<D2>,
{
    type Output = E2::Output;
    fn resolve(self, dependency: (D1, D2)) -> Self::Output {
        let (d1, d2) = dependency;
        self.0.resolve(d1).resolve(d2)
    }
}

/// Flatten an effect returning an effect into a single effect
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Collapse<E>(E);

impl<D, E> Effect<D> for Collapse<E>
where
    E: Effect<(D, D)>,
    D: Clone,
{
    type Output = E::Output;
    fn resolve(self, dependency: D) -> Self::Output {
        self.0.resolve((dependency.clone(), dependency))
    }
}

/// Flatten an effect returning an effect (with the same dependency) into a single effect
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct FlatCollapse<E1>(E1);

impl<D, E1, E2> Effect<D> for FlatCollapse<E1>
where
    D: Clone,
    E1: Effect<D, Output = E2>,
    E2: Effect<D>,
{
    type Output = E2::Output;
    fn resolve(self, dependency: D) -> Self::Output {
        self.0.resolve(dependency.clone()).resolve(dependency)
    }
}

/// Flatten an effect returning an effect (with the same dependency) into a single effect, where the dependency is a mutable reference.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct FlatCollapseMut<E1>(E1);

impl<'a, D, E1, E2> Effect<&'a mut D> for FlatCollapseMut<E1>
where
    E1: for<'b> Effect<&'b mut D, Output = E2>,
    E2: for<'b> Effect<&'b mut D>,
    <E2 as Effect<&'a mut D>>::Output: 'static,
{
    type Output = <E2 as Effect<&'a mut D>>::Output;
    fn resolve(self, dependency: &'a mut D) -> Self::Output {
        self.0.resolve(dependency).resolve(dependency)
    }
}

/// Provide the left dependency of an effect with a 2-tuple dependency.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ProvideLeft<E, D> {
    effect: E,
    left_dependency: D,
}

impl<D1, D2, E> Effect<D2> for ProvideLeft<E, D1>
where
    E: Effect<(D1, D2)>,
{
    type Output = E::Output;
    fn resolve(self, dependency: D2) -> Self::Output {
        self.effect.resolve((self.left_dependency, dependency))
    }
}

/// Provide the right dependency of an effect with a 2-tuple dependency.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ProvideRight<E, D> {
    effect: E,
    right_dependency: D,
}

impl<D1, D2, E> Effect<D1> for ProvideRight<E, D2>
where
    E: Effect<(D1, D2)>,
{
    type Output = E::Output;
    fn resolve(self, dependency: D1) -> Self::Output {
        self.effect.resolve((dependency, self.right_dependency))
    }
}
