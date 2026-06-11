use crate::Effect;

// Balanced Either type that implements Effect.
pub enum Either<L, R> {
    Left(L),
    Right(R),
}
impl<L, R, D> Effect<D> for Either<L, R>
where
    L: Effect<D>,
    R: Effect<D, Output = L::Output>,
{
    type Output = L::Output;
    fn resolve(self, dependency: D) -> Self::Output {
        match self {
            Either::Left(l) => l.resolve(dependency),
            Either::Right(r) => r.resolve(dependency),
        }
    }
}
