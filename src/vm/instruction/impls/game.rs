use super::*;

pub fn set_light_level(ctx: Context) -> Result<()> {
    let v = ctx.vm_state.data_stack.pop()?.into_int()?;
    log_a1!(ctx.vm_state, &v);
    log_stub!(ctx.vm_state);
    Ok(())
}