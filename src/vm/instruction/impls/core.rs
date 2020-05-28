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
        let r = f(l.partial_cmp(&r, ctx.prg.strings())?);
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
    binary_op(ctx, |l, r, ctx| l.add(r, ctx.prg.strings()))
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

pub fn call(ctx: Context) -> Result<()> {
    let proc_id = ctx.prg.data_stack.pop()?.into_int()? as ProcedureId;
    let body_pos = {
        let proc = ctx.prg.program.proc(proc_id)
            .ok_or_else(|| Error::BadProcedureId(proc_id))?;
        if proc.flags.contains(ProcedureFlag::Import) {
            return Err(Error::Misc("calling imported procedure is not supported".into()));
        }
        proc.body_pos as i32
    };
    ctx.prg.jump(body_pos)?;
    log_a1r1!(ctx.prg, proc_id, body_pos);
    Ok(())
}

pub fn div(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, ctx| l.div(r, ctx.prg.strings()))
}

pub fn const_float(ctx: Context) -> Result<()> {
    let v = ctx.prg.next_f32()?;
    ctx.prg.data_stack.push(Value::Float(v))?;
    log_r1!(ctx.prg, ctx.prg.data_stack.top().unwrap());
    Ok(())
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
    let s = ctx.prg.data_stack.pop()?.into_string(ctx.prg.strings())?;
    log_a1!(ctx.prg, s);
    info!(target: "vault13::vm::debug", "{}", s.display());
    Ok(())
}

pub fn dtoa(ctx: Context) -> Result<()> {
    let v = ctx.prg.data_stack.pop()?;
    ctx.prg.return_stack.push(v)?;
    log_r1!(ctx.prg, ctx.prg.return_stack.top().unwrap());
    Ok(())
}

pub fn dup(ctx: Context) -> Result<()> {
    let v = ctx.prg.data_stack.pop()?;
    ctx.prg.data_stack.push(v.clone())?;
    ctx.prg.data_stack.push(v)?;
    log_a1!(ctx.prg, ctx.prg.data_stack.top().unwrap());
    Ok(())
}

pub fn equal(ctx: Context) -> Result<()> {
    cmp_test(ctx, |o| o == Some(Ordering::Equal))
}

pub fn fetch(ctx: Context) -> Result<()> {
    let id = ctx.prg.data_stack.pop()?.into_int()?;
    let v = ctx.prg.base_val(id as usize)?.clone();
    ctx.prg.data_stack.push(v)?;
    log_a1r1!(ctx.prg, id, ctx.prg.data_stack.top().unwrap());
    Ok(())
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
        Err(Error::Misc(format!("external variable `{}` already exists", name.display()).into()))
    }
}

pub fn fetch_external(ctx: Context) -> Result<()> {
    let name = ctx.prg.data_stack.pop()?.into_string(&ctx.prg.names())?;

    let r = ctx.ext.external_vars.get(&name).cloned()
        .ok_or_else(|| Error::BadExternalVar(name.clone()))?
        .unwrap_or(0.into());
    ctx.prg.data_stack.push(r)?;

    log_a1r1!(ctx.prg, &name, ctx.prg.data_stack.top().unwrap());
    Ok(())
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

pub fn mod_(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, ctx| l.rem(r, ctx.prg.strings()))
}

pub fn mul(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, ctx| l.mul(r, ctx.prg.strings()))
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
    let v = ctx.prg.return_stack.pop()?.into_int()?;
    ctx.prg.set_base_encoded(v);
    log_r1!(ctx.prg, &ctx.prg.base);
    Ok(())
}

fn pop_flags0(ctx: &mut Context) -> Result<(i32, i32, i32)> {
    let unk17 = ctx.prg.data_stack.pop()?.into_int()?;
    let unk19 = ctx.prg.data_stack.pop()?.into_int()?;
    let flags = ctx.prg.data_stack.pop()?.into_int()?;
    Ok((unk17, unk19, flags))
}

pub fn pop_flags(mut ctx: Context) -> Result<()> {
    let (unk17, unk19, flags) = pop_flags0(&mut ctx)?;
    log_a3!(ctx.prg, unk17, unk19, flags);
    Ok(())
}

pub fn pop_flags_exit(mut ctx: Context) -> Result<()> {
    let (unk17, unk19, flags) = pop_flags0(&mut ctx)?;
    let pos = ctx.prg.return_stack.pop()?.into_int()?;
    ctx.prg.jump(pos)?;
    log_a4!(ctx.prg, unk17, unk19, flags, pos);
    Err(Error::Halted)
}

pub fn pop_flags_return(mut ctx: Context) -> Result<()> {
    let (unk17, unk19, flags) = pop_flags0(&mut ctx)?;
    let pos = ctx.prg.return_stack.pop()?.into_int()?;
    ctx.prg.jump(pos)?;
    log_a4!(ctx.prg, unk17, unk19, flags, pos);
    Ok(())
}

pub fn pop_return(ctx: Context) -> Result<()> {
    let pos = ctx.prg.return_stack.pop()?.into_int()?;
    ctx.prg.jump(pos)?;
    log_a1!(ctx.prg, &pos);
    Ok(())
}

pub fn pop_to_base(ctx: Context) -> Result<()> {
    let base = ctx.prg.base()?;
    ctx.prg.data_stack.truncate(base)?;
    log_r1!(ctx.prg, base);
    Ok(())
}

pub fn push_base(ctx: Context) -> Result<()> {
    let arg_count = ctx.prg.data_stack.pop()?.into_int()?;
    let new_base = ctx.prg.data_stack.len() as isize - arg_count as isize;
    if new_base < 0 {
        return Err(Error::BadValue(BadValue::Content));
    }
    let new_base = new_base as usize;
    ctx.prg.return_stack.push(Value::Int(ctx.prg.base_encoded()))?;
    ctx.prg.base = Some(new_base);
    debug!("{:?}: new base: {}", ctx.prg.opcode.unwrap().0, new_base);
    log_a1r2!(ctx.prg, &arg_count, ctx.prg.return_stack.top().unwrap(), &new_base);
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

pub fn script_overrides(ctx: Context) -> Result<()> {
    ctx.prg.instr_state.script_overrides = true;
    log_!(ctx.prg);
    Ok(())
}

pub fn store(ctx: Context) -> Result<()> {
    let id = ctx.prg.data_stack.pop()?.into_int()? as usize;
    let value = ctx.prg.data_stack.pop()?;
    *ctx.prg.base_val_mut(id)? = value;
    log_a2!(ctx.prg, id, ctx.prg.base_val(id).unwrap());
    Ok(())
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
        .ok_or_else(|| Error::BadExternalVar(name.clone()))?;
    *v = Some(value);
    log_a2!(ctx.prg, &name, v);
    Ok(())
}

pub fn sub(ctx: Context) -> Result<()> {
    binary_op(ctx, |l, r, ctx| l.sub(r, ctx.prg.strings()))
}

pub fn swap(ctx: Context) -> Result<()> {
    let v1 = ctx.prg.data_stack.pop()?;
    let v2 = ctx.prg.data_stack.pop()?;
    ctx.prg.data_stack.push(v1)?;
    ctx.prg.data_stack.push(v2)?;
    log_a2!(ctx.prg,
        ctx.prg.data_stack.get(ctx.prg.data_stack.len() - 2).unwrap(),
        ctx.prg.data_stack.top().unwrap());
    Ok(())
}

pub fn swapa(ctx: Context) -> Result<()> {
    let v1 = ctx.prg.return_stack.pop()?;
    let v2 = ctx.prg.return_stack.pop()?;
    ctx.prg.return_stack.push(v1)?;
    ctx.prg.return_stack.push(v2)?;
    log_a2!(ctx.prg,
        ctx.prg.return_stack.get(ctx.prg.return_stack.len() - 2).unwrap(),
        ctx.prg.return_stack.top().unwrap());
    Ok(())
}

pub fn unimplemented(ctx: Context) -> Result<()> {
    Err(Error::UnimplementedOpcode(ctx.prg.opcode.unwrap().0))
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