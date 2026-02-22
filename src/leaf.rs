use crate::{Context, Status};

pub trait ActionHandler<A> {
    fn execute(&mut self, action: &A, ctx: &mut Context) -> Status;
}

pub trait ConditionHandler<C> {
    fn check(&self, condition: &C, ctx: &Context) -> bool;
}
