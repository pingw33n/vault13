#[macro_use] mod macros;
mod core;
mod game;

pub use self::core::*;
pub use self::game::*;

use super::Context;
use super::value::*;
use super::super::*;

fn binary_op(ctx: Context, f: impl FnOnce(Value, Value, &Context) -> Result<Value>) -> Result<()> {
    let right = ctx.prg.data_stack.pop()?;
    let left = ctx.prg.data_stack.pop()?;
    let r = f(left.clone(), right.clone(), &ctx)?;
    ctx.prg.data_stack.push(r)?;
    log_a2r1!(ctx.prg,
        left.resolved(ctx.prg.strings()).unwrap(),
        right.resolved(ctx.prg.strings()).unwrap(),
        ctx.prg.data_stack.top().unwrap());
    Ok(())
}

fn unary_op(ctx: Context, f: impl FnOnce(Value, &Context) -> Result<Value>) -> Result<()> {
    let v = ctx.prg.data_stack.pop()?;
    let r = f(v.clone(), &ctx)?;
    ctx.prg.data_stack.push(r)?;
    log_a1r1!(ctx.prg, v, ctx.prg.data_stack.top().unwrap());
    Ok(())
}
