use crate::Effect;

// Balanced Either type that implements Effect.
pub enum Either<L, R> {
    Left(L),
    Right(R),
}
impl<L, R, T, D> Effect<D> for Either<L, R>
where
    L: Effect<D, Output = T>,
    R: Effect<D, Output = T>,
{
    type Output = T;
    fn resolve(self, dependency: D) -> Self::Output {
        match self {
            Either::Left(l) => l.resolve(dependency),
            Either::Right(r) => r.resolve(dependency),
        }
    }
}
