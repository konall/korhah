use crate::{compat::VariableBounds, Id};

use core::marker::PhantomData;

/// A typed handle to a variable belonging to the reactive system
#[derive(educe::Educe)]
#[educe(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variable<T: VariableBounds> {
    pub(crate) id: Id,
    pub(crate) _t: PhantomData<T>,
}

impl<T: VariableBounds> Into<Option<VariableId>> for Variable<T> {
    fn into(self) -> Option<VariableId> {
        Some(VariableId(self.id))
    }
}

/// An untyped handle to a variable belonging to the reactive system
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariableId(pub(crate) Id);
