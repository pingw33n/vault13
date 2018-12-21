#[macro_use] mod macros;
mod core;
mod game;

pub use self::core::*;
pub use self::game::*;

use super::Context;
use super::value::*;
use super::super::*;

fn binary_op(ctx: Context, f: impl FnOnce(Value, Value, &Context) -> Result<Value>) -> Result<()> {
    let right = ctx.vm_state.data_stack.pop()?;
    let left = ctx.vm_state.data_stack.pop()?;
    let r = f(left.clone(), right.clone(), &ctx)?;
    ctx.vm_state.data_stack.push(r)?;
    log_a2r1!(ctx.vm_state,
        left.resolved(&ctx.vm_state.strings).unwrap(),
        right.resolved(&ctx.vm_state.strings).unwrap(),
        ctx.vm_state.data_stack.top().unwrap());
    Ok(())
}

fn unary_op(ctx: Context, f: impl FnOnce(Value, &Context) -> Result<Value>) -> Result<()> {
    let v = ctx.vm_state.data_stack.pop()?;
    let r = f(v.clone(), &ctx)?;
    ctx.vm_state.data_stack.push(r)?;
    log_a1r1!(ctx.vm_state, v, ctx.vm_state.data_stack.top().unwrap());
    Ok(())
}
