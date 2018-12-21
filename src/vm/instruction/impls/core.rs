use std::cmp::Ordering;

use super::*;

fn cmp_test(ctx: Context, f: impl FnOnce(Option<Ordering>) -> bool) -> Result<()> {
    binary_op(ctx, |l, r, ctx| {
        let r = f(l.partial_cmp(&r, &ctx.vm_state.strings)?);
        Ok(Value::boolean(r))
    })
}

pub fn add(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, ctx| l.add(r, &ctx.vm_state.strings))
}

pub fn and(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, _| Ok(Value::boolean(l.test() && r.test())))
}

pub fn atod(ctx: Context) -> Result<()> {
    let v = ctx.vm_state.return_stack.pop()?;
    ctx.vm_state.data_stack.push(v)?;
    log_r1!(ctx.vm_state, ctx.vm_state.data_stack.top().unwrap());
    Ok(())
}

pub fn const_int(ctx: Context) -> Result<()> {
    let v = ctx.vm_state.next_i32()?;
    ctx.vm_state.data_stack.push(Value::Int(v))?;
    log_r1!(ctx.vm_state, ctx.vm_state.data_stack.top().unwrap());
    Ok(())
}

pub fn const_string(ctx: Context) -> Result<()> {
    let v = ctx.vm_state.next_i32()?;
    if v >= 0 {
        ctx.vm_state.data_stack.push(Value::String(StringValue::Indirect(v as usize)))?;
        log_r1!(ctx.vm_state, ctx.vm_state.data_stack.top().unwrap());
        Ok(())
    } else {
        Err(Error::BadInstruction)
    }
}

pub fn debug_msg(ctx: Context) -> Result<()> {
    let s = ctx.vm_state.data_stack.pop()?.into_string(&ctx.vm_state.strings)?;
    log_a1!(ctx.vm_state, s);
    info!(target: "vault13::vm::debug", "{}", s);
    Ok(())
}

pub fn dtoa(ctx: Context) -> Result<()> {
    let v = ctx.vm_state.data_stack.pop()?;
    ctx.vm_state.return_stack.push(v)?;
    log_r1!(ctx.vm_state, ctx.vm_state.return_stack.top().unwrap());
    Ok(())
}

pub fn equal(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o == Some(Ordering::Equal))
}

pub fn fetch_global(ctx: Context) -> Result<()> {
    let id = ctx.vm_state.data_stack.pop()?.into_int()?;
    let v = ctx.vm_state.global(id as usize)?.clone();
    ctx.vm_state.data_stack.push(v)?;
    log_a1r1!(ctx.vm_state, id, ctx.vm_state.data_stack.top().unwrap());
    Ok(())
}

pub fn exit_prog(ctx: Context) -> Result<()> {
    log!(ctx.vm_state);
    Err(Error::Halted)
}

pub fn export_var(ctx: Context) -> Result<()> {
    let name = ctx.vm_state.data_stack.pop()?;
    let name = name.into_string(&ctx.vm_state.names)?;
    if !ctx.ext.external_vars.contains_key(&name) {
        log_a1!(ctx.vm_state, &name);
        ctx.ext.external_vars.insert(name, Value::Null);
        Ok(())
    } else {
        Err(Error::Misc(format!("external variable `{}` already exists", name).into()))
    }
}

pub fn global_var(ctx: Context) -> Result<()> {
    let id = ctx.vm_state.data_stack.pop()?.into_int()?;
    let v = Value::Int(if let Some(&v) = ctx.ext.global_vars.get(id as usize) {
        v
    } else {
        warn!("GlobalVar: attempted to get undefined global var {}", id);
        -1
    });
    ctx.vm_state.data_stack.push(v)?;
    log_a1r1!(ctx.vm_state, &id, ctx.vm_state.data_stack.top().unwrap());
    Ok(())
}

pub fn greater(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o == Some(Ordering::Greater))
}

pub fn greater_equal(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o == Some(Ordering::Greater) || o == Some(Ordering::Equal))
}

pub fn jmp(ctx: Context) -> Result<()> {
    let pos = ctx.vm_state.data_stack.pop()?.into_int()?;
    ctx.vm_state.jump(pos)?;
    log_a1!(ctx.vm_state, &pos);
    Ok(())
}

pub fn if_(ctx: Context) -> Result<()> {
    let cond = ctx.vm_state.data_stack.pop()?;
    let jump_pos = ctx.vm_state.data_stack.pop()?.into_int()?;
    if cond.test() {
        ctx.vm_state.jump(jump_pos)?;
    }
    log_a1r1!(ctx.vm_state, cond, jump_pos);
    Ok(())
}

pub fn less(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o == Some(Ordering::Less))
}

pub fn less_equal(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o == Some(Ordering::Less) || o == Some(Ordering::Equal))
}

pub fn negate(ctx: Context) -> Result<()> {
    unary_op(ctx, |v, _| v.neg())
}

pub fn noop(ctx: Context) -> Result<()> {
    log!(ctx.vm_state);
    Ok(())
}

pub fn not(ctx: Context) -> Result<()> {
    unary_op(ctx, |v, _| v.not())
}

pub fn not_equal(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o != Some(Ordering::Equal))
}

pub fn or(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, _| Ok(Value::boolean(l.test() || r.test())))
}

pub fn pop(ctx: Context) -> Result<()> {
    let v = ctx.vm_state.data_stack.pop()?;
    log_r1!(ctx.vm_state, v);
    Ok(())
}

pub fn pop_base(ctx: Context) -> Result<()> {
    ctx.vm_state.base = ctx.vm_state.return_stack.pop()?.into_int()? as isize;
    log_r1!(ctx.vm_state, &ctx.vm_state.base);
    Ok(())
}

pub fn pop_flags_exit(ctx: Context) -> Result<()> {
    let pos = ctx.vm_state.data_stack.pop()?.into_int()?;
    ctx.vm_state.jump(pos)?;
    log_a1!(ctx.vm_state, pos);
    Err(Error::Halted)
}

pub fn pop_flags_return(ctx: Context) -> Result<()> {
    let flags = ctx.vm_state.return_stack.pop()?.into_int()?;
    log_r1!(ctx.vm_state, flags);
    Ok(())
}

pub fn pop_return(ctx: Context) -> Result<()> {
    let pos = ctx.vm_state.return_stack.pop()?.into_int()?;
    ctx.vm_state.jump(pos)?;
    log_a1!(ctx.vm_state, &pos);
    Ok(())
}

pub fn pop_to_base(ctx: Context) -> Result<()> {
    if ctx.vm_state.base < 0 {
        return Err(Error::Misc("base is not set".into()));
    }
    let base = ctx.vm_state.base as usize;
    ctx.vm_state.data_stack.truncate(base)?;
    log_r1!(ctx.vm_state, &base);
    Ok(())
}

pub fn push_base(ctx: Context) -> Result<()> {
    let arg_count = ctx.vm_state.data_stack.pop()?.into_int()?;
    let new_base = ctx.vm_state.data_stack.len() as isize - arg_count as isize;
    if new_base >= 0 {
        ctx.vm_state.return_stack.push(Value::Int(ctx.vm_state.base as i32))?;
        ctx.vm_state.base = new_base;
        log_a1r2!(ctx.vm_state, &arg_count, ctx.vm_state.return_stack.top().unwrap(), &new_base);
        Ok(())
    } else {
        Err(Error::BadValue(BadValue::Content))
    }
}

pub fn set_global(ctx: Context) -> Result<()> {
    let global_base = ctx.vm_state.data_stack.len();
    ctx.vm_state.global_base = Some(global_base);
    log_r1!(ctx.vm_state, global_base);
    Ok(())
}

pub fn set_global_var(ctx: Context) -> Result<()> {
    let value = ctx.vm_state.data_stack.pop()?.into_int()?;
    let id = ctx.vm_state.data_stack.pop()?.into_int()?;
    log_a2!(ctx.vm_state, id, value);
    if let Some(v) = ctx.ext.global_vars.get_mut(id as usize) {
        *v = value;
    } else {
        warn!("GlobalVar: attempted to set undefined global var {} = {}", id, value);
    }
    Ok(())
}

pub fn store_global(ctx: Context) -> Result<()> {
    let id = ctx.vm_state.data_stack.pop()?.into_int()? as usize;
    let value = ctx.vm_state.data_stack.pop()?;
    *ctx.vm_state.global_mut(id)? = value;
    log_a2!(ctx.vm_state, id, ctx.vm_state.global(id).unwrap());
    Ok(())
}

pub fn store_external(ctx: Context) -> Result<()> {
    let name = ctx.vm_state.data_stack.pop()?;
    let value = ctx.vm_state.data_stack.pop()?;
    let name = name.into_string(&ctx.vm_state.names)?;
    let v = ctx.ext.external_vars.get_mut(&name)
        .ok_or_else(|| Error::Misc(format!("external variable `{}` doesn't exist", name).into()))?;
    *v = value;
    log_a2!(ctx.vm_state, &name, v);
    Ok(())
}

pub fn swapa(ctx: Context) -> Result<()> {
    let dv = ctx.vm_state.data_stack.pop()?;
    let rv = ctx.vm_state.return_stack.pop()?;
    ctx.vm_state.data_stack.push(rv)?;
    ctx.vm_state.return_stack.push(dv)?;
    log_a2!(ctx.vm_state,
        ctx.vm_state.return_stack.top().unwrap(),
        ctx.vm_state.data_stack.top().unwrap());
    Ok(())
}

pub fn unimplemented(_ctx: Context) -> Result<()> {
    Err(Error::UnimplementedOpcode)
}

pub fn while_(ctx: Context) -> Result<()> {
    let done = ctx.vm_state.data_stack.pop()?;
    if !done.test() {
        let jump_pos = ctx.vm_state.data_stack.pop()?.into_int()?;
        ctx.vm_state.jump(jump_pos)?;
        log_a1r1!(ctx.vm_state, done, jump_pos);
    } else {
        log_a1!(ctx.vm_state, done);
    }
    Ok(())
}