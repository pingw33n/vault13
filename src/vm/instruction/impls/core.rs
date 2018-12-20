use super::*;

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

pub fn dtoa(ctx: Context) -> Result<()> {
    let v = ctx.vm_state.data_stack.pop()?;
    ctx.vm_state.return_stack.push(v)?;
    log_r1!(ctx.vm_state, ctx.vm_state.return_stack.top().unwrap());
    Ok(())
}

pub fn exit_prog(ctx: Context) -> Result<()> {
    log!(ctx.vm_state);
    Err(Error::Halted)
}

pub fn export_var(ctx: Context) -> Result<()> {
    let name = ctx.vm_state.data_stack.pop()?;
    let name = name.into_string(&ctx.vm_state.names)?;
    if !ctx.ext.vars.contains_key(&name) {
        log_a1!(ctx.vm_state, &name);
        ctx.ext.vars.insert(name, Value::Null);
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
        warn!("GlobalVar: global var {} doesn't exist", id);
        -1
    });
    ctx.vm_state.data_stack.push(v)?;
    log_a1r1!(ctx.vm_state, &id, ctx.vm_state.data_stack.top().unwrap());
    Ok(())
}

pub fn jmp(ctx: Context) -> Result<()> {
    let pos = ctx.vm_state.data_stack.pop()?.into_int()?;
    ctx.vm_state.jump(pos)?;
    log_a1!(ctx.vm_state, &pos);
    Ok(())
}

pub fn less(ctx: Context) -> Result<()> {
    let left = ctx.vm_state.data_stack.pop()?;
    let right = ctx.vm_state.data_stack.pop()?;
    let r = Value::Int(match &left {
        Value::Null => return Err(Error::BadValue(BadValue::Type)),
        Value::Int(left) => match &right {
            Value::Null => return Err(Error::BadValue(BadValue::Type)),
            Value::Int(right) => left < right,
            Value::Float(right) => (*left as f32) < *right,
            Value::String(right) => {
                let left = left.to_string();
                let right = right.clone().resolve(&ctx.vm_state.strings)?;
                &left < &right
            }
            Value::Object(_) => return Err(Error::BadValue(BadValue::Type)),
        }
        Value::Float(left) => match &right {
            Value::Null => return Err(Error::BadValue(BadValue::Type)),
            Value::Int(right) => *left < (*right as f32),
            Value::Float(right) => left < right,
            Value::String(right) => {
                let left = left.to_string();
                let right = right.clone().resolve(&ctx.vm_state.strings)?;
                &left < &right
            }
            Value::Object(_) => return Err(Error::BadValue(BadValue::Type)),
        }
        Value::String(left) => match &right {
            Value::Null => return Err(Error::BadValue(BadValue::Type)),
            Value::Int(right) => {
                let left = left.clone().resolve(&ctx.vm_state.strings)?;
                let right = right.to_string();
                &*left < &right
            },
            Value::Float(right) => {
                let left = left.clone().resolve(&ctx.vm_state.strings)?;
                let right = right.to_string();
                &*left < &right
            },
            Value::String(right) => {
                let left = left.clone().resolve(&ctx.vm_state.strings)?;
                let right = right.clone().resolve(&ctx.vm_state.strings)?;
                left.to_lowercase() < right.to_lowercase()
            }
            Value::Object(_) => return Err(Error::BadValue(BadValue::Type)),
        }
        Value::Object(_) => return Err(Error::BadValue(BadValue::Type)),
    } as i32);
    ctx.vm_state.data_stack.push(r)?;
    log_a2r1!(ctx.vm_state, left, right, ctx.vm_state.data_stack.top().unwrap());
    Ok(())
}

pub fn negate(ctx: Context) -> Result<()> {
    let v = ctx.vm_state.data_stack.pop()?;
    let r = match &v {
        Value::Int(v) => Value::Int(-v),
        Value::Float(v) => Value::Float(-v),
        _ => return Err(Error::BadValue(BadValue::Type)),
    };
    ctx.vm_state.data_stack.push(r)?;
    log_a1r1!(ctx.vm_state, &v, ctx.vm_state.data_stack.top().unwrap());
    Ok(())
}

pub fn noop(ctx: Context) -> Result<()> {
    log!(ctx.vm_state);
    Ok(())
}

pub fn pop_base(ctx: Context) -> Result<()> {
    ctx.vm_state.base = ctx.vm_state.return_stack.pop()?.into_int()? as isize;
    log_r1!(ctx.vm_state, &ctx.vm_state.base);
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
    let global_base = ctx.vm_state.data_stack.len() as isize;
    ctx.vm_state.global_base = global_base;
    log_r1!(ctx.vm_state, &global_base);
    Ok(())
}

pub fn store_external(ctx: Context) -> Result<()> {
    let name = ctx.vm_state.data_stack.pop()?;
    let value = ctx.vm_state.data_stack.pop()?;
    let name = name.into_string(&ctx.vm_state.names)?;
    let v = ctx.ext.vars.get_mut(&name)
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