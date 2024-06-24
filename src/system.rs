use crate::{
    compat::{Cell, FnBounds, Guard, Handler, Rc, Recipe, Value, VariableBounds},
    events::{Created, Creating, Deleted, Deleting, Read, Reading, Updated, Updating},
    listener::{Listener, Vote, Votes},
    variable::{Variable, VariableId},
    Id,
};

use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use core::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use ahash::RandomState;
use indexmap::IndexMap;

/// The reactive system
#[derive(Clone, Default)]
pub struct System<'x>(pub(crate) Rc<Cell<SystemInner<'x>>>);

#[derive(Default)]
pub(crate) struct SystemInner<'x> {
    next_id: Id,
    id_pool: Vec<Id>,
    /// while a tracking ID is set, system reads establish a dependency between the tracked variable and the variable being read
    tracking_id: Option<Id>,
    dependencies: BTreeMap<Id, BTreeSet<Id>>,
    values: BTreeMap<Id, Value>,
    recipes: BTreeMap<Id, Recipe<'x>>,
    listeners: BTreeMap<TypeId, BTreeMap<Option<Id>, IndexMap<Id, Handler<'x>, RandomState>>>,
}

impl<'x> System<'x> {
    fn hold(&self) -> Guard<'_, SystemInner<'x>> {
        #[cfg(not(feature = "unsync"))]
        {
            self.0.lock()
        }
        #[cfg(feature = "unsync")]
        {
            self.0.borrow_mut()
        }
    }

    /// Create a new variable in the reactive system.\
    /// The `recipe` parameter computes the value of the variable- it receives a read-only handle to the system, and the
    /// previous value of the variable ([`None`] on creation, [`Some`] for subsequent updates).\
    /// Variables that are read within the recipe are automatically tracked- whenever any of them are updated, a re-run of
    /// the recipe is triggered to update this new variable's value.\
    /// The creation of this variable (and any subsequent updates to it) can only be cancelled _after_ the recipe has already
    /// been run, so care should be taken to avoid unwanted side-effects- however, since the recipe is read-only, it should be
    /// difficult to accidentally go wrong, as it will by default be idempotent.
    ///
    /// Returns:
    /// - [`Err`], if the action was cancelled
    /// - an [`Ok`] value containing the new variable, otherwise
    ///
    /// # Example
    /// ```
    /// let mut system = korhah::System::default();
    ///
    /// let a = system.create(|_, _| 0).expect("no cancelling listeners registered");
    /// let b = system.create(move |s, _| {
    ///     // `b`'s dependency on `a` is automatically tracked here
    ///     s
    ///         .read(a, |v| *v + 1)
    ///         .expect("no cancelling listeners registered")
    ///         .expect("`a` was not deleted")
    /// }).expect("no cancelling listeners registered");
    /// assert_eq!(Ok(Some(1)), system.read(b, |v| *v));
    ///
    /// _ = system.update(a, |v| *v += 1);
    /// assert_eq!(Ok(Some(2)), system.read(b, |v| *v));
    /// ```
    pub fn create<T, F>(&mut self, recipe: F) -> Result<Variable<T>, ()>
    where
        T: VariableBounds,
        F: Fn(&System<'x>, Option<T>) -> T + FnBounds + 'x,
    {
        SystemInner::create(self.clone(), recipe)
    }

    /// Read the value of a variable in the reactive system.\
    /// The `callback` parameter computes the value to be returned from this function- it receives a read-only reference to the target variable's current value.
    ///
    /// Returns:
    /// - [`Err`], if the action was cancelled
    /// - an [`Ok`] value containing [`None`], if the target variable doesn't exist
    /// - an [`Ok`] value containing a [`Some`] value containing the result of the passed callback, otherwise
    ///
    /// # Example
    /// ```
    /// let mut system = korhah::System::default();
    ///
    /// let a = system.create(|_, _| 0).expect("no cancelling listeners registered");
    ///
    /// assert_eq!(Ok(Some(true)), system.read(a, |v| *v < 5));
    ///
    /// _ = system.delete(a);
    /// assert_eq!(Ok(None), system.read(a, |v| *v + 1));
    /// ```
    pub fn read<T, F, S>(&self, variable: Variable<T>, callback: F) -> Result<Option<S>, ()>
    where
        T: VariableBounds,
        F: FnOnce(&T) -> S,
    {
        SystemInner::read(self.clone(), variable, callback)
    }

    /// Update the value of a variable in the reactive system.\
    /// The `callback` parameter performs the update, and optionally returns a value to the caller- it receives
    /// a mutable reference to the target variable's current value.
    ///
    /// Returns:
    /// - [`Err`], if the action was cancelled
    /// - an [`Ok`] value containing [`None`], if the target variable doesn't exist
    /// - an [`Ok`] value containing a [`Some`] value containing the result of the passed callback, otherwise
    ///
    /// # Example
    /// ```
    /// let mut system = korhah::System::default();
    ///
    /// let a = system.create(|_, _| 0).expect("no cancelling listeners registered");
    ///
    /// assert_eq!(Ok(Some(())), system.update(a, |v| *v += 1));
    /// assert_eq!(Ok(Some(1)), system.read(a, |v| *v));
    ///
    /// _ = system.delete(a);
    /// assert_eq!(Ok(None), system.update(a, |v| *v += 2));
    /// ```
    pub fn update<T, F, S>(&mut self, variable: Variable<T>, callback: F) -> Result<Option<S>, ()>
    where
        T: VariableBounds,
        F: FnOnce(&mut T) -> S,
    {
        SystemInner::update(self.clone(), variable, callback)
    }

    /// Remove a variable from the reactive system.
    ///
    /// Returns:
    /// - [`Err`], if the action was cancelled (including if deleting the target variable would leave dangling references)
    /// - an [`Ok`] value containing [`None`], if the target variable doesn't exist
    /// - an [`Ok`] value containing the most recent value of the deleted variable, otherwise
    ///
    /// # Example
    /// ```
    /// let mut system = korhah::System::default();
    ///
    /// let a = system.create(|_, _| 0).expect("no cancelling listeners registered");
    /// let b = system.create(move |s, _| {
    ///     s
    ///         .read(a, |v| *v + 1)
    ///         .expect("no cancelling listeners registered")
    ///         .expect("`a` was not deleted")
    /// }).expect("no cancelling listeners registered");
    ///
    /// // can't delete `a` as `b` depends on it
    /// assert_eq!(Err(()), system.delete(a));
    ///
    /// // if we delete `b` first, we can then delete `a` as it has no dependents
    /// assert_eq!(Ok(Some(1)), system.delete(b));
    /// assert_eq!(Ok(Some(0)), system.delete(a));
    ///
    /// // now `a` doesn't exist
    /// assert_eq!(Ok(None), system.delete(a));
    /// ```
    pub fn delete<T>(&mut self, variable: Variable<T>) -> Result<Option<T>, ()>
    where
        T: VariableBounds,
    {
        SystemInner::delete(self.clone(), variable)
    }

    /// Register a handler that will be called when a certain event is triggered in the reactive system.\
    /// A [`None`] target listens for an event in the global scope, whereas a [`Some`] target listens for an event on that specific variable.\
    /// The `handler` parameter receives a read-write handle to the system, as well as:
    /// - a reference to the triggered event
    /// - a mutable reference that can be used to cast the handler's vote on the triggered event (see [`Vote`], [`Votes`])
    /// - a mutable reference that can be used to abort the triggered event
    ///
    /// Events are uniquely identified by their type, so type annotations are always required for the event argument of the handler.
    ///
    /// Returns:
    /// - [`None`], if the target variable doesn't exist
    /// - a [`Some`] value containing the new listener, otherwise
    ///
    /// # Example
    /// ```
    /// struct CustomEvent {
    ///     n: usize,
    /// }
    ///
    /// let mut system = korhah::System::default();
    ///
    /// let listener = system.listen(None, move |_, e: &CustomEvent, vote, abort| {
    ///     if e.n == 1 {
    ///         *vote = korhah::Vote::Cancel;
    ///     } else if e.n == 2 {
    ///         *abort = true;
    ///     }
    /// }).expect("can always listen in the global scope");
    ///
    /// let votes = system.emit(None, &CustomEvent { n: 0 }).expect("not aborted if n == 0");
    /// assert!(votes.cancel <= votes.proceed);
    ///
    /// let votes = system.emit(None, &CustomEvent { n: 1 }).expect("not aborted if n == 1");
    /// assert!(votes.cancel >= votes.proceed);
    ///
    /// assert!(system.emit(None, &CustomEvent { n: 2 }).is_err());
    /// ```
    pub fn listen<E, F>(
        &self,
        target: impl Into<Option<VariableId>>,
        handler: F,
    ) -> Option<Listener<E>>
    where
        E: 'static,
        F: Fn(&mut System<'x>, &E, &mut Vote, &mut bool) + FnBounds + 'x,
    {
        SystemInner::listen(self.clone(), target, handler)
    }

    /// Trigger the given event in the reactive system.\
    /// A [`None`] target triggers an event in the global scope, whereas a [`Some`] target triggers an event on that specific variable.
    ///
    /// Returns:
    /// - [`Err`], if any of the triggered handlers aborted the event
    /// - an [`Ok`] value containing the consensus among the triggered handlers on the event's effects, otherwise (see [`Votes`])
    ///
    /// # Example
    /// ```
    /// struct CustomEvent;
    ///
    /// let mut system = korhah::System::default();
    ///
    /// let triggered = system.create(|_, _| false).expect("no cancelling listeners registered");
    /// system.listen(None, move |s, _: &CustomEvent, _, abort| {
    ///     if s.read(triggered, |v| *v)
    ///         .expect("no cancelling listeners registered")
    ///         .expect("`a` exists")
    ///     {
    ///         *abort = true;
    ///     } else {
    ///         _ = s.update(triggered, |v| *v = true);
    ///     }
    /// });
    ///
    /// assert!(system.emit(None, &CustomEvent).is_ok());
    /// assert!(system.emit(None, &CustomEvent).is_err());
    /// ```
    pub fn emit<E>(&mut self, target: impl Into<Option<VariableId>>, event: &E) -> Result<Votes, ()>
    where
        E: 'static,
    {
        SystemInner::emit(self.clone(), target, event)
    }

    /// Remove the given event listener from the reactive system.
    ///
    /// Returns:
    /// - [`None`], if the target event listener doesn't exist
    /// - [`Some`], otherwise
    ///
    /// # Example
    /// ```
    /// struct CustomEvent;
    ///
    /// let mut system = korhah::System::default();
    ///
    /// let x = system.create(|_, _| 0).expect("no cancelling listeners registered");
    /// let listener = system.listen(None, move |s, _: &CustomEvent, _, _| {
    ///     _ = s.update(x, |v| *v += 1);
    /// }).expect("can always listen in the global scope");
    ///
    /// _ = system.emit(None, &CustomEvent);
    /// assert_eq!(Ok(Some(1)), system.read(x, |v| *v));
    /// _ = system.emit(None, &CustomEvent);
    /// assert_eq!(Ok(Some(2)), system.read(x, |v| *v));
    ///
    /// assert!(system.silence(listener).is_some());
    ///
    /// _ = system.emit(None, &CustomEvent);
    /// assert_eq!(Ok(Some(2)), system.read(x, |v| *v));
    ///
    /// assert!(system.silence(listener).is_none());
    /// ```
    pub fn silence<E>(&mut self, listener: Listener<E>) -> Option<()>
    where
        E: 'static,
    {
        SystemInner::silence(self.clone(), listener)
    }
}

impl<'x> SystemInner<'x> {
    /// add a new variable to the reactive system
    fn create<T, F>(mut this: System<'x>, recipe: F) -> Result<Variable<T>, ()>
    where
        T: VariableBounds,
        F: Fn(&System<'x>, Option<T>) -> T + FnBounds + 'x,
    {
        // previously-deleted IDs are reused if possible, otherwise new IDs are allocated by incrementing a global counter
        let id = {
            // use an intermediate variable to avoid deadlock
            let reusable_id = this.hold().id_pool.pop();
            reusable_id.unwrap_or_else(|| {
                let id = this.hold().next_id;
                this.hold().next_id += 1;
                id
            })
        };

        // ensure the new variable's dependencies, if any, are tracked
        this.hold().tracking_id = Some(id);
        let value = recipe(&this, None);
        this.hold().tracking_id = None;

        let event = Creating { value };
        // since the variable is not yet created, it's impossible to listen for its local events at this point, so
        // the `Creating` event is only emitted in the global scope
        if this
            .emit(None, &event)
            .map(|votes| votes.cancel > votes.proceed)
            .unwrap_or(true)
        {
            // since the `Creating` event has been cancelled, the ID we selected hasn't ended up being used, so we free it
            this.hold().id_pool.push(id);
            return Err(());
        }

        // reclaim the newly-created value after having temporarily loaned it to the `Creating` event
        let value = event.value;
        // store the type-erased initial value
        this.hold().values.insert(id, Box::new(value));
        // we have to wrap the recipe somewhat, in order to supply the previous value of the variable as an argument to it
        // and to type-erase its return value
        this.hold().recipes.insert(
            id,
            Rc::new(move |s| {
                let prev = s
                    .hold()
                    .values
                    .remove(&id)
                    .and_then(|v| v.downcast().ok())
                    .map(|v| *v);
                Box::new(recipe(s, prev))
            }),
        );

        let variable = Variable {
            id,
            _t: PhantomData,
        };

        // same as the `Creating` event, the `Created` event is emitted only in the global scope as it's impossible to
        // listen for it locally ahead of time
        // we don't care if the `Created` event is cancelled, as it doesn't prevent any subsequent actions
        _ = this.emit(None, &Created { source: variable });

        Ok(variable)
    }

    /// read the value of a variable in the reactive system
    fn read<T, F, S>(
        mut this: System<'x>,
        variable: Variable<T>,
        callback: F,
    ) -> Result<Option<S>, ()>
    where
        T: VariableBounds,
        F: FnOnce(&T) -> S,
    {
        if !this.hold().values.contains_key(&variable.id) {
            // the target variable doesn't exist so we ignore this request
            return Ok(None);
        }

        // the `Reading` event is cancellable
        if this
            .emit(variable, &Reading)
            .map(|votes| votes.cancel > votes.proceed)
            .unwrap_or(true)
        {
            return Err(());
        }

        // store the tracking ID serparately to avoid deadlock
        let dependent = this.hold().tracking_id;
        if let Some(dependent) = dependent {
            // this variable is being read as part of a new variable's recipe, so we track the dependency
            // in order to trigger updates when the new variable is changed, and to prevent dangling references
            this.hold()
                .dependencies
                .entry(variable.id)
                .or_default()
                .insert(dependent);
        }

        // compute the result of the passed callback
        let ret = callback(
            // this type system should prevent downcasting errors here, so `unwrap` is used here to preserve the semantic meaning of
            // an `Ok(Some)`, `Ok(None)`, or `Err` return value from this function
            this.hold()
                .values
                .get(&variable.id)
                .unwrap()
                .downcast_ref()
                .unwrap(),
        );

        // we don't care if `Read` events are cancelled as there are no subsequent actions to take
        _ = this.emit(variable, &Read);

        Ok(Some(ret))
    }

    /// update the value of a variable in the reactive system
    fn update<T, F, S>(
        mut this: System<'x>,
        variable: Variable<T>,
        callback: F,
    ) -> Result<Option<S>, ()>
    where
        T: VariableBounds,
        F: FnOnce(&mut T) -> S,
    {
        if !this.hold().values.contains_key(&variable.id) {
            // the target variable doesn't exist so we ignore this request
            return Ok(None);
        }

        // the `Updating` event is cancellable
        if this
            .emit(variable, &Updating)
            .map(|votes| votes.cancel > votes.proceed)
            .unwrap_or(true)
        {
            return Err(());
        }

        // invoke the callback that will update the variable
        let ret = callback(
            // this type system should prevent downcasting errors here, so `unwrap` is used here to preserve the semantic meaning of
            // an `Ok(Some)`, `Ok(None)`, or `Err` return value from this function
            this.hold()
                .values
                .get_mut(&variable.id)
                .unwrap()
                .downcast_mut()
                .unwrap(),
        );

        // store this variable's dependents (if any) separately to avoid deadlock
        let dependents = this
            .hold()
            .dependencies
            .get(&variable.id)
            .cloned()
            .unwrap_or_default();
        // we must recompute the values of any variables that depend on the just-changed variable
        // since we don't have access to the type of the dependent variables, we have to manually recompute them instead of
        // being able to use the `update` function
        for dependent in dependents {
            // the `Updating` event for the dependent variables can be cancelled as usual
            if this
                .emit(VariableId(dependent), &Updating)
                .map(|votes| votes.cancel > votes.proceed)
                .unwrap_or(true)
            {
                continue;
            }

            // update the value of the dependent variable
            let recipe = this
                .hold()
                .recipes
                .get(&dependent)
                .cloned()
                .expect("all variables have a recipe");
            let value = recipe(&this);
            this.hold().values.insert(dependent, value);

            // we don't care if this `Updated` event is cancelled as there are no subsequent actions to take for this dependent variable
            _ = this.emit(VariableId(dependent), &Updated);
        }

        // we don't care if this `Updated` event is cancelled as there are no subsequent actions to take
        _ = this.emit(variable, &Updated);

        Ok(Some(ret))
    }

    /// remove a variable from the reactive system
    fn delete<T>(mut this: System<'x>, variable: Variable<T>) -> Result<Option<T>, ()>
    where
        T: VariableBounds,
    {
        if !this.hold().values.contains_key(&variable.id) {
            // the target variable doesn't exist so we ignore this request
            return Ok(None);
        }

        // cancel the deletion if the value of any other variables depends on this one, as
        // that would otherwise leave a dangling reference
        if this
            .hold()
            .dependencies
            .get(&variable.id)
            .map(|deps| !deps.is_empty())
            .unwrap_or_default()
        {
            return Err(());
        }

        // the `Deleting` event is cancellable
        if this
            .emit(variable, &Deleting)
            .map(|votes| votes.cancel > votes.proceed)
            .unwrap_or(true)
        {
            return Err(());
        }

        // wipe the resources associated with the deleted variable
        this.hold().dependencies.remove(&variable.id);
        this.hold().dependencies.values_mut().for_each(|deps| {
            deps.remove(&variable.id);
        });
        this.hold().recipes.remove(&variable.id);
        this.hold().listeners.values_mut().for_each(|listeners| {
            listeners.remove(&Some(variable.id));
        });
        this.hold().id_pool.push(variable.id);

        // this type system should prevent downcasting errors here, so `unwrap` is used here to preserve the semantic meaning of
        // an `Ok(Some)`, `Ok(None)`, or `Err` return value from this function
        let value = *this
            .hold()
            .values
            .remove(&variable.id)
            .unwrap()
            .downcast()
            .unwrap();

        // we don't care if `Read` events are cancelled as there are no subsequent actions to take
        _ = this.emit(None, &Deleted { _source: variable });

        Ok(Some(value))
    }

    /// register a function to be called
    fn listen<E, F>(
        this: System<'x>,
        target: impl Into<Option<VariableId>>,
        handler: F,
    ) -> Option<Listener<E>>
    where
        E: 'static,
        F: Fn(&mut System<'x>, &E, &mut Vote, &mut bool) + FnBounds + 'x,
    {
        // extract the ID of the passed target, if any
        let target_id = target.into().map(|VariableId(id)| id);

        if target_id
            .map(|id| !this.hold().values.contains_key(&id))
            .unwrap_or_default()
        {
            // the target variable doesn't exist so we ignore this request
            return None;
        }

        // we have to wrap the passed handler in order to upcast the event type, so that handlers for different
        // event types can be treated the same in the system
        let handler = Rc::new(
            move |system: &mut System<'x>, event: &dyn Any, vote: &mut Vote, abort: &mut bool| {
                if let Some(event) = event.downcast_ref() {
                    handler(system, event, vote, abort);
                }
            },
        );

        // listener IDs are allocated from the same pool as variables
        let id = {
            let reusable_id = this.hold().id_pool.pop();
            reusable_id.unwrap_or_else(|| {
                let id = this.hold().next_id;
                this.hold().next_id += 1;
                id
            })
        };

        // store the event handler
        this.hold()
            .listeners
            .entry(TypeId::of::<E>())
            .or_default()
            .entry(target_id)
            .or_insert(IndexMap::with_hasher(RandomState::new()))
            .insert(id, handler);

        Some(Listener {
            id,
            target: target_id,
            _e: PhantomData,
        })
    }

    /// trigger an event, optionally on a given target
    fn emit<E>(
        mut this: System<'x>,
        target: impl Into<Option<VariableId>>,
        event: &E,
    ) -> Result<Votes, ()>
    where
        E: 'static,
    {
        // extract the ID of the passed target, if any
        let target_id = target.into().map(|VariableId(id)| id);

        // gather the relevant handlers for this event & target
        let handlers = this
            .hold()
            .listeners
            .get(&event.type_id())
            .and_then(|targets| targets.get(&target_id))
            .into_iter()
            .flatten()
            .map(|(_, handler)| handler)
            .cloned()
            .collect::<Vec<_>>();

        let mut votes = Votes::default();
        for handler in handlers {
            // by default this handler will proceed without affecting subsequent ones
            let mut vote = Vote::Abstain;
            let mut abort = false;
            handler(&mut this, event, &mut vote, &mut abort);

            // if aborted, subsqeuent handlers are skipped
            if abort {
                return Err(());
            }

            // votes are tallied following the execution of alll handlers, so we continue on
            match vote {
                Vote::Abstain => votes.abstain += 1,
                Vote::Cancel => votes.cancel += 1,
                Vote::Proceed => votes.proceed += 1,
            }
        }

        Ok(votes)
    }

    /// removes an event listener
    fn silence<E>(this: System<'x>, listener: Listener<E>) -> Option<()>
    where
        E: 'static,
    {
        this.hold()
            .listeners
            .get_mut(&TypeId::of::<E>())
            .and_then(|targets| targets.get_mut(&listener.target))
            .and_then(|handlers| handlers.shift_remove(&listener.id))
            .map(|_| ())
    }
}
