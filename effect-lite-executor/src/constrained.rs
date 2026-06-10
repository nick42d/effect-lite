use core::any::Any;

#[derive(Eq, PartialEq, Debug)]
pub struct Constraint<Md> {
    pub(crate) constraint_type: ConstraitType<Md>,
}

impl<Md> Constraint<Md> {
    pub fn new_block_same_type() -> Self {
        Self {
            constraint_type: ConstraitType::BlockSameType,
        }
    }
    pub fn new_kill_same_type() -> Self {
        Self {
            constraint_type: ConstraitType::KillSameType,
        }
    }
    pub fn new_block_matching_metadata(metadata: Md) -> Self {
        Self {
            constraint_type: ConstraitType::BlockMatchingMetatdata(metadata),
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum ConstraitType<Md> {
    BlockSameType,
    KillSameType,
    BlockMatchingMetatdata(Md),
}
struct EffectConstrained<E, Md> {
    constraint: Constraint<Md>,
    effect: E,
    metadata: allocator_api2::vec::Vec<Md>,
}
struct ConstrainedEffectExecutor<E, T, D> {
    inner: super::Executor<E, T, D, allocator_api2::alloc::Global>,
    info_list: allocator_api2::vec::Vec<()>,
}
impl<E, T, D> ConstrainedEffectExecutor<E, T, D> {
    pub fn new(dependencies: D) -> Self {
        let inner = crate::Executor::new(dependencies, allocator_api2::alloc::Global)
            .with_on_push_cb(|_| ());
        Self {
            inner,
            info_list: allocator_api2::vec![],
        }
    }
    pub fn push_ref<S, Md>(&mut self, effect: EffectConstrained<E, Md>)
    where
        E: for<'b> effect_lite::Effect<&'b D, Output = S> + Any + 'static,
        S: futures::Stream<Item = T> + Any + 'static,
        T: 'static,
    {
        let EffectConstrained {
            constraint,
            effect,
            metadata,
        } = effect;
        self.inner.push_ref(effect, allocator_api2::alloc::Global);
    }
    pub fn get_next(&mut self) -> crate::GetNext<T, allocator_api2::alloc::Global> {
        self.inner.get_next()
    }
    pub fn get_dependencies(&self) -> &D {
        &self.inner.dependencies
    }
    pub fn get_dependencies_mut(&mut self) -> &mut D {
        &mut self.inner.dependencies
    }
}
