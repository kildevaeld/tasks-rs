#![deny(clippy::all)]
#[macro_use]
mod macros;

mod and;
mod and_then;
mod boxed;
mod filter;
mod filter_pipe;
mod generic;
mod map;
mod map_err;
// mod map_rejection;
mod middleware;
mod or;
mod pass;
mod pipe;
mod task;
mod task_ext;
mod task_state;
mod unify;
mod unroll;
pub mod util;

pub use self::{
    and::*, and_then::*, boxed::*, filter::*, filter_pipe::*, generic::*, map::*, map_err::*,
    middleware::*, or::*, pass::*, pipe::*, task::*, task_ext::*, task_state::*, unify::*,
    unroll::*,
};
