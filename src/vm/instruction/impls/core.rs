use log::*;
use std::cmp::Ordering;

use super::*;

#[derive(Clone, Copy, Debug)]
enum PersistentVarScope {
    Local,
    Map,
    Global,
}

impl PersistentVarScope {
    pub fn get(self, ctx: &crate::vm::Context, id: usize) -> Option<i32> {
        use PersistentVarScope::*;
        match self {
            Local => ctx.local_vars.get(id),
            Map => ctx.map_vars.get(id),
            Global => ctx.global_vars.get(id),
        }.cloned()
    }

    #[must_use]
    pub fn set(self, ctx: &mut crate::vm::Context, id: usize, value: i32) -> bool {
        use PersistentVarScope::*;
        if let Some(v) = match self {
            Local => ctx.local_vars.get_mut(id),
            Map => ctx.map_vars.get_mut(id),
            Global => ctx.global_vars.get_mut(id),
        } {
            *v = value;
            true
        } else {
            false
        }
    }
}

fn cmp_test(ctx: Context, f: impl FnOnce(Option<Ordering>) -> bool) -> Result<()> {
    binary_op(ctx, |l, r, ctx| {
        let r = f(l.partial_cmp(&r, &ctx.prg.strings())?);
        Ok(r.into())
    })
}

fn persistent_var(ctx: Context, scope: PersistentVarScope) -> Result<()> {
    let id = ctx.prg.data_stack.pop()?.into_int()?;
    let v = Value::Int(if let Some(v) = scope.get(ctx.ext, id as usize) {
        v
    } else {
        warn!("{:?}: attempted to get undefined {:?} var {}",
            ctx.prg.opcode.unwrap().0, scope, id);
        0
    });
    ctx.prg.data_stack.push(v)?;
    log_a1r1!(ctx.prg, &id, ctx.prg.data_stack.top().unwrap());
    Ok(())
}

fn set_persistent_var(ctx: Context, scope: PersistentVarScope) -> Result<()> {
    let value = ctx.prg.data_stack.pop()?.into_int()?;
    let id = ctx.prg.data_stack.pop()?.into_int()?;
    log_a2!(ctx.prg, id, value);
    if !scope.set(ctx.ext, id as usize, value) {
        warn!("{:?}: attempted to set undefined {:?} var {} = {}",
            ctx.prg.opcode.unwrap().0, scope, id, value);
    }
    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub fn add(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, ctx| l.add(r, &ctx.prg.strings()))
}

pub fn and(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, _| Ok((l.test() && r.test()).into()))
}

pub fn atod(ctx: Context) -> Result<()> {
    let v = ctx.prg.return_stack.pop()?;
    ctx.prg.data_stack.push(v)?;
    log_r1!(ctx.prg, ctx.prg.data_stack.top().unwrap());
    Ok(())
}

pub fn bwand(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, _| l.bwand(r))
}

pub fn bwnot(ctx: Context) -> Result<()> {
    unary_op(ctx, |v, _| v.bwnot())
}

pub fn bwor(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, _| l.bwor(r))
}

pub fn bwxor(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, _| l.bwxor(r))
}

pub fn const_int(ctx: Context) -> Result<()> {
    let v = ctx.prg.next_i32()?;
    ctx.prg.data_stack.push(Value::Int(v))?;
    log_r1!(ctx.prg, ctx.prg.data_stack.top().unwrap());
    Ok(())
}

pub fn const_string(ctx: Context) -> Result<()> {
    let v = ctx.prg.next_i32()?;
    if v >= 0 {
        ctx.prg.data_stack.push(Value::String(StringValue::Indirect(v as usize)))?;
        log_r1!(ctx.prg, ctx.prg.data_stack.top().unwrap());
        Ok(())
    } else {
        Err(Error::BadInstruction)
    }
}

pub fn debug_msg(ctx: Context) -> Result<()> {
    let s = ctx.prg.data_stack.pop()?.into_string(&ctx.prg.strings())?;
    log_a1!(ctx.prg, s);
    info!(target: "vault13::vm::debug", "{}", s);
    Ok(())
}

pub fn dtoa(ctx: Context) -> Result<()> {
    let v = ctx.prg.data_stack.pop()?;
    ctx.prg.return_stack.push(v)?;
    log_r1!(ctx.prg, ctx.prg.return_stack.top().unwrap());
    Ok(())
}

pub fn equal(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o == Some(Ordering::Equal))
}

pub fn fetch_global(ctx: Context) -> Result<()> {
    let id = ctx.prg.data_stack.pop()?.into_int()?;
    let v = ctx.prg.global(id as usize)?.clone();
    ctx.prg.data_stack.push(v)?;
    log_a1r1!(ctx.prg, id, ctx.prg.data_stack.top().unwrap());
    Ok(())
}

pub fn exit_prog(ctx: Context) -> Result<()> {
    log_!(ctx.prg);
    Err(Error::Halted)
}

pub fn export_var(ctx: Context) -> Result<()> {
    let name = ctx.prg.data_stack.pop()?;
    let name = name.into_string(&ctx.prg.names())?;
    if !ctx.ext.external_vars.contains_key(&name) {
        log_a1!(ctx.prg, &name);
        ctx.ext.external_vars.insert(name, None);
        Ok(())
    } else {
        Err(Error::Misc(format!("external variable `{}` already exists", name).into()))
    }
}

pub fn global_var(ctx: Context) -> Result<()> {
    persistent_var(ctx, PersistentVarScope::Global)
}

pub fn greater(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o == Some(Ordering::Greater))
}

pub fn greater_equal(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o == Some(Ordering::Greater) || o == Some(Ordering::Equal))
}

pub fn jmp(ctx: Context) -> Result<()> {
    let pos = ctx.prg.data_stack.pop()?.into_int()?;
    ctx.prg.jump(pos)?;
    log_a1!(ctx.prg, &pos);
    Ok(())
}

pub fn if_(ctx: Context) -> Result<()> {
    let cond = ctx.prg.data_stack.pop()?;
    let jump_pos = ctx.prg.data_stack.pop()?.into_int()?;
    if !cond.test() {
        ctx.prg.jump(jump_pos)?;
    }
    log_a1r1!(ctx.prg, cond, jump_pos);
    Ok(())
}

pub fn less(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o == Some(Ordering::Less))
}

pub fn less_equal(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o == Some(Ordering::Less) || o == Some(Ordering::Equal))
}

pub fn local_var(ctx: Context) -> Result<()> {
    persistent_var(ctx, PersistentVarScope::Local)
}

pub fn map_var(ctx: Context) -> Result<()> {
    persistent_var(ctx, PersistentVarScope::Map)
}

pub fn negate(ctx: Context) -> Result<()> {
    unary_op(ctx, |v, _| v.neg())
}

pub fn noop(ctx: Context) -> Result<()> {
    log_!(ctx.prg);
    Ok(())
}

pub fn not(ctx: Context) -> Result<()> {
    unary_op(ctx, |v, _| Ok(v.not()))
}

pub fn not_equal(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o != Some(Ordering::Equal))
}

pub fn or(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, _| Ok((l.test() || r.test()).into()))
}

pub fn pop(ctx: Context) -> Result<()> {
    let v = ctx.prg.data_stack.pop()?;
    log_r1!(ctx.prg, v);
    Ok(())
}

pub fn pop_base(ctx: Context) -> Result<()> {
    ctx.prg.base = ctx.prg.return_stack.pop()?.into_int()? as isize;
    log_r1!(ctx.prg, &ctx.prg.base);
    Ok(())
}

pub fn pop_flags_exit(ctx: Context) -> Result<()> {
    let pos = ctx.prg.data_stack.pop()?.into_int()?;
    ctx.prg.jump(pos)?;
    log_a1!(ctx.prg, pos);
    Err(Error::Halted)
}

pub fn pop_flags_return(ctx: Context) -> Result<()> {
    let flags = ctx.prg.return_stack.pop()?.into_int()?;
    log_r1!(ctx.prg, flags);
    Ok(())
}

pub fn pop_return(ctx: Context) -> Result<()> {
    let pos = ctx.prg.return_stack.pop()?.into_int()?;
    ctx.prg.jump(pos)?;
    log_a1!(ctx.prg, &pos);
    Ok(())
}

pub fn pop_to_base(ctx: Context) -> Result<()> {
    if ctx.prg.base < 0 {
        return Err(Error::Misc("base is not set".into()));
    }
    let base = ctx.prg.base as usize;
    ctx.prg.data_stack.truncate(base)?;
    log_r1!(ctx.prg, &base);
    Ok(())
}

pub fn push_base(ctx: Context) -> Result<()> {
    let arg_count = ctx.prg.data_stack.pop()?.into_int()?;
    let new_base = ctx.prg.data_stack.len() as isize - arg_count as isize;
    if new_base >= 0 {
        ctx.prg.return_stack.push(Value::Int(ctx.prg.base as i32))?;
        ctx.prg.base = new_base;
        log_a1r2!(ctx.prg, &arg_count, ctx.prg.return_stack.top().unwrap(), &new_base);
        Ok(())
    } else {
        Err(Error::BadValue(BadValue::Content))
    }
}

pub fn self_obj(ctx: Context) -> Result<()> {
    ctx.prg.data_stack.push(Value::Object(ctx.ext.self_obj))?;
    log_r1!(ctx.prg, ctx.prg.data_stack.top().unwrap());

    Ok(())
}

pub fn set_global(ctx: Context) -> Result<()> {
    let global_base = ctx.prg.data_stack.len();
    ctx.prg.global_base = Some(global_base);
    log_r1!(ctx.prg, global_base);
    Ok(())
}

pub fn set_global_var(ctx: Context) -> Result<()> {
    set_persistent_var(ctx, PersistentVarScope::Global)
}

pub fn set_local_var(ctx: Context) -> Result<()> {
    set_persistent_var(ctx, PersistentVarScope::Local)
}

pub fn set_map_var(ctx: Context) -> Result<()> {
    set_persistent_var(ctx, PersistentVarScope::Map)
}

pub fn store_global(ctx: Context) -> Result<()> {
    let id = ctx.prg.data_stack.pop()?.into_int()? as usize;
    let value = ctx.prg.data_stack.pop()?;
    *ctx.prg.global_mut(id)? = value;
    log_a2!(ctx.prg, id, ctx.prg.global(id).unwrap());
    Ok(())
}

pub fn store_external(ctx: Context) -> Result<()> {
    let name = ctx.prg.data_stack.pop()?;
    let value = ctx.prg.data_stack.pop()?;
    let name = name.into_string(&ctx.prg.names())?;
    let v = ctx.ext.external_vars.get_mut(&name)
        .ok_or_else(|| Error::Misc(format!("external variable `{}` doesn't exist", name).into()))?;
    *v = Some(value);
    log_a2!(ctx.prg, &name, v);
    Ok(())
}

pub fn swapa(ctx: Context) -> Result<()> {
    let dv = ctx.prg.data_stack.pop()?;
    let rv = ctx.prg.return_stack.pop()?;
    ctx.prg.data_stack.push(rv)?;
    ctx.prg.return_stack.push(dv)?;
    log_a2!(ctx.prg,
        ctx.prg.return_stack.top().unwrap(),
        ctx.prg.data_stack.top().unwrap());
    Ok(())
}

pub fn unimplemented(_ctx: Context) -> Result<()> {
    Err(Error::UnimplementedOpcode)
}

pub fn while_(ctx: Context) -> Result<()> {
    let done = ctx.prg.data_stack.pop()?;
    if !done.test() {
        let jump_pos = ctx.prg.data_stack.pop()?.into_int()?;
        ctx.prg.jump(jump_pos)?;
        log_a1r1!(ctx.prg, done, jump_pos);
    } else {
        log_a1!(ctx.prg, done);
    }
    Ok(())
}