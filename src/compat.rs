pub use compat::*;

#[cfg(not(feature = "unsync"))]
mod compat {
    use alloc::boxed::Box;
    use core::any::Any;

    pub(crate) use ::spin::{Mutex as Cell, MutexGuard as Guard};
    pub(crate) use alloc::sync::Arc as Rc;

    use crate::{listener::Vote, system::System};

    pub trait VariableBounds: Any + Send + Sync {}
    impl<T: Any + Send + Sync> VariableBounds for T {}

    pub trait FnBounds: Send + Sync {}
    impl<F: Send + Sync> FnBounds for F {}

    pub(crate) type Value = Box<dyn Any + Send + Sync>;

    pub(crate) type Handler<'a> =
        Rc<dyn Fn(&mut System<'a>, &dyn Any, &mut Vote, &mut bool) + Send + Sync + 'a>;
    pub(crate) type Recipe<'a> = Rc<dyn Fn(&System<'a>) -> Value + Send + Sync + 'a>;
}

#[cfg(feature = "unsync")]
mod compat {
    use alloc::boxed::Box;
    use core::any::Any;

    pub(crate) use alloc::rc::Rc;
    pub(crate) use core::cell::{RefCell as Cell, RefMut as Guard};

    use crate::{listener::Vote, system::System};

    pub trait VariableBounds: Any {}
    impl<T: Any> VariableBounds for T {}

    pub trait FnBounds {}
    impl<F> FnBounds for F {}

    pub(crate) type Value = Box<dyn Any>;

    pub(crate) type Handler<'a> = Rc<dyn Fn(&mut System<'a>, &dyn Any, &mut Vote, &mut bool) + 'a>;
    pub(crate) type Recipe<'a> = Rc<dyn Fn(&System<'a>) -> Value + 'a>;
}
