use crate::{compat::VariableBounds, variable::Variable};

/// A variable is about to be created.\
/// If this event is cancelled, the variable will not be created.\
/// Since it's impossible to listen for this event prior to the variable existing, it's emitted in the global scope.
#[derive(educe::Educe)]
#[educe(Clone, Copy)]
pub struct Creating<T: VariableBounds> {
    /// The value that will be given to the newly-created variable
    pub value: T,
}

/// A variable has just been created.\
/// Cancelling this event has no effect.\
/// Since it's impossible to listen for this event prior to the variable existing, it's emitted in the global scope.
#[derive(educe::Educe)]
#[educe(Clone, Copy)]
pub struct Created<T: VariableBounds> {
    /// The variable that was just created
    pub source: Variable<T>,
}

/// The targeted variable is about to be read.\
/// If this event is cancelled, the variable will not be read.
#[derive(Clone, Copy)]
pub struct Reading;

/// The targeted variable has just been read.\
/// Cancelling this event has no effect.
#[derive(Clone, Copy)]
pub struct Read;

/// The targeted variable is about to be updated.\
/// If this event is cancelled, the variable will not be updated.
#[derive(Clone, Copy)]
pub struct Updating;

/// The targeted variable has just been updated.\
/// Cancelling this event has no effect.
#[derive(Clone, Copy)]
pub struct Updated;

/// The targeted variable is about to be deleted.\
/// If this event is cancelled, the variable will not be deleted.
#[derive(Clone, Copy)]
pub struct Deleting;

/// A variable has just been deleted.\
/// Cancelling this event has no effect.\
/// Since the targeted variable no longer exists in the system, this event is emitted in the global scope.\
/// The deleted variable is provided in case any cleanup is required external to the system- it should not be used in any system operations.
#[derive(educe::Educe)]
#[educe(Clone, Copy)]
pub struct Deleted<T: VariableBounds> {
    /// The variable that was just deleted- it should not be used for any interactions with the system
    pub _source: Variable<T>,
}
