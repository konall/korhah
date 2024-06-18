use crate::Id;

use core::marker::PhantomData;

/// A typed handle to a listener in the reactive system
#[derive(educe::Educe)]
#[educe(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Listener<E: 'static> {
    pub(crate) id: Id,
    pub(crate) target: Option<Id>,
    pub(crate) _e: PhantomData<E>,
}

/// Represents a certain event handler's preference as to whether or not the effects from its corresponding event should be followed through
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Vote {
    /// Choose to abstain from voting
    #[default]
    Abstain,
    /// Vote to cancel the effects of an event
    Cancel,
    /// Vote to proceed with the effects of an event
    Proceed,
}

/// Represents the consensus among event handlers as to whether or not the effects from their corresponding event should be followed through.\
/// Effects of built-in events are followed through if the number of votes to proceed >= the number of votes to cancel.\
/// Custom events decide their own criteria for acting on these results, if at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Votes {
    /// The number of event handlers who chose to abstain from voting
    pub abstain: usize,
    /// The number of event handlers who voted to cancel the event
    pub cancel: usize,
    /// The number of event handlers who voted to proceed with the event
    pub proceed: usize,
}
