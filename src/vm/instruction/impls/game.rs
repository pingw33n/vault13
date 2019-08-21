use enum_map_derive::Enum;
use enum_primitive_derive::Primitive;
use log::*;
use num_traits::FromPrimitive;
use std::cmp;

use super::*;
use crate::asset::proto::Pid;
use crate::game::object::Object;
use crate::game::script::Sid;
use crate::graphics::geometry::hex::Direction;
use crate::sequence::Sequence;
use crate::sequence::chain::Chain;
use crate::graphics::EPoint;

#[derive(Clone, Copy, Debug, Enum, Eq, Hash, Ord, PartialEq, PartialOrd, Primitive)]
enum Metarule {
    SignalEndGame   = 13,
    TestFirstrun    = 14,
    Elevator        = 15,
    PartyCount      = 16,
    AreaKnown       = 17,
    WhoOnDrugs      = 18,
    MapKnown        = 19,
    IsLoadgame      = 22,
    CarCurrentTown  = 30,
    GiveCarToParty  = 31,
    GiveCarGas      = 32,
    SkillCheckTag   = 40,
    DropAllInven    = 42,
    InvenUnwieldWho = 43,
    GetWorldmapXpos = 44,
    GetWorldmapYpos = 45,
    CurrentTown     = 46,
    LanguageFilter  = 47,
    ViolenceFilter  = 48,
    WDamageType     = 49,
    CritterBarters  = 50,
    CritterKillType = 51,
    CarTrunkSetAnim = 52,
    CarTrunkGetAnim = 53,
}

#[derive(Clone, Copy, Debug, Enum, Eq, Hash, Ord, PartialEq, PartialOrd, Primitive)]
enum Metarule3 {
    ClrFixedTimedEvents = 100,
    MarkSubtile         = 101,
    SetWmMusic          = 102,
    GetKillCount        = 103,
    MarkMapEntrance     = 104,
    WmSubtileState      = 105,
    TileGetNextCritter  = 106,
    ArtSetBaseFidNum    = 107,
    TileSetCenter       = 108,
    AiGetChemUseValue   = 109,
    WmCarIsOutOfGas     = 110,
    MapTargetLoadArea   = 111,
}

#[derive(Clone, Copy, Debug, Enum, Eq, Hash, Ord, PartialEq, PartialOrd, Primitive)]
enum RegAnimFuncOp {
    Begin = 1,
    Clear = 2,
    End = 3,
}

pub fn add_mult_objs_to_inven(ctx: Context) -> Result<()> {
    let count = ctx.prg.data_stack.pop()?.into_int()?;
    let count = if count > 99999 {
        500
    } else if count < 0 {
        1
    } else {
        count
    };
    let item = ctx.prg.data_stack.pop()?.coerce_into_object()?;
    let target = ctx.prg.data_stack.pop()?.coerce_into_object()?;
    log_a3!(ctx.prg, target, item, count);
    log_stub!(ctx.prg);
    Ok(())
}

pub fn create_object_sid(ctx: Context) -> Result<()> {
    let sid = ctx.prg.data_stack.pop()?.into_int()?;
    let sid = if sid >= 0 {
        Some(Sid::from_packed(sid as u32)
            .ok_or(Error::BadValue(BadValue::Content))?)
    } else {
        None
    };

    let elevation = ctx.prg.data_stack.pop()?.into_int()? as u32;
    let tile_num = cmp::max(ctx.prg.data_stack.pop()?.into_int()?, 0) as u32;

    let pid = ctx.prg.data_stack.pop()?.into_int()?;
    let pid = Pid::from_packed(pid as u32)
        .ok_or(Error::BadValue(BadValue::Content))?;

    // FIXME add proper impl
    let fid = ctx.ext.world.proto_db().proto(pid).unwrap().fid;
    let pos = ctx.ext.world.map_grid().hex().from_linear_inv(tile_num);
    let pos = pos.elevated(elevation);
    let obj = Object::new(fid, Some(pid), Some(pos));
    let objh = ctx.ext.world.insert_object(obj);

    ctx.prg.data_stack.push(Value::Object(Some(objh)))?;

    log_a4r1!(ctx.prg, pid, tile_num, elevation, sid, objh);
    log_stub!(ctx.prg);

    Ok(())
}

pub fn critter_add_trait(ctx: Context) -> Result<()> {
    let value = ctx.prg.data_stack.pop()?.into_int()?;
    let sub_kind = ctx.prg.data_stack.pop()?.into_int()?;
    let kind = ctx.prg.data_stack.pop()?.into_int()?;
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?;

    let r = -1;
    ctx.prg.data_stack.push(r.into())?;

    log_a4r1!(ctx.prg, obj, kind, sub_kind, value, r);
    log_stub!(ctx.prg);
    Ok(())
}

pub fn destroy_object(ctx: Context) -> Result<()> {
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?;
    log_a1!(ctx.prg, obj);
    log_stub!(ctx.prg);
    Ok(())
}

pub fn dude_obj(ctx: Context) -> Result<()> {
    let obj = ctx.ext.world.dude_obj();
    ctx.prg.data_stack.push(Value::Object(obj))?;
    log_r1!(ctx.prg, obj);
    Ok(())
}

pub fn game_time(ctx: Context) -> Result<()> {
    let r = ctx.ext.world.game_time.as_decis();
    ctx.prg.data_stack.push(Value::Int(r as i32))?;
    log_r1!(ctx.prg, r);
    Ok(())
}

pub fn game_time_hour(ctx: Context) -> Result<()> {
    let time = ctx.ext.world.game_time;
    let r = 100 * time.hour() as u32 + time.minute() as u32;
    ctx.prg.data_stack.push(Value::Int(r as i32))?;
    log_r1!(ctx.prg, r);
    Ok(())
}

pub fn game_time_in_seconds(ctx: Context) -> Result<()> {
    let r = ctx.ext.world.game_time.as_seconds();
    ctx.prg.data_stack.push(Value::Int(r as i32))?;
    log_r1!(ctx.prg, r);
    Ok(())
}

pub fn get_day(ctx: Context) -> Result<()> {
    let r = ctx.ext.world.game_time.day();
    ctx.prg.data_stack.push(Value::Int(r as i32))?;
    log_r1!(ctx.prg, r);
    Ok(())
}

pub fn get_month(ctx: Context) -> Result<()> {
    let r = ctx.ext.world.game_time.month();
    ctx.prg.data_stack.push(Value::Int(r as i32))?;
    log_r1!(ctx.prg, r);
    Ok(())
}

pub fn metarule(ctx: Context) -> Result<()> {
    let value = ctx.prg.data_stack.pop()?.into_int()?;
    let id = ctx.prg.data_stack.pop()?.into_int()?;

    use self::Metarule::*;
    let mr = Metarule::from_i32(id);
    let r = if let Some(mr) = mr {
        match mr {
            SignalEndGame   => 0,
            TestFirstrun    => 1,
            Elevator        => 0,
            PartyCount      => 0,
            AreaKnown       => 1,
            WhoOnDrugs      => 0,
            MapKnown        => 1,
            IsLoadgame      => 0,
            CarCurrentTown  => 0,
            GiveCarToParty  => 0,
            GiveCarGas      => 0,
            SkillCheckTag   => 0,
            DropAllInven    => 0,
            InvenUnwieldWho => 0,
            GetWorldmapXpos => 0,
            GetWorldmapYpos => 0,
            CurrentTown     => 0,
            LanguageFilter  => 0,
            ViolenceFilter  => 0,
            WDamageType     => 0,
            CritterBarters  => 0,
            CritterKillType => 0,
            CarTrunkSetAnim => 0,
            CarTrunkGetAnim => 0,
        }
    } else {
        error!("unknown Metarule ID {}", id);
        0
    };

    ctx.prg.data_stack.push(Value::Int(r))?;

    if let Some(mr) = mr {
        log_a2r1!(ctx.prg, mr, value, ctx.prg.data_stack.top().unwrap());
    } else {
        log_a2r1!(ctx.prg, id, value, ctx.prg.data_stack.top().unwrap());
    }
    log_stub!(ctx.prg);

    Ok(())
}

pub fn metarule3(ctx: Context) -> Result<()> {
    let v3 = ctx.prg.data_stack.pop()?;
    let v2 = ctx.prg.data_stack.pop()?;
    let v1 = ctx.prg.data_stack.pop()?;
    let id = ctx.prg.data_stack.pop()?.into_int()?;

    use self::Metarule3::*;
    let mr = Metarule3::from_i32(id);
    let r = if let Some(mr) = mr {
        match mr {
            ClrFixedTimedEvents => 0,
            MarkSubtile         => 0,
            SetWmMusic          => 0,
            GetKillCount        => 0,
            MarkMapEntrance     => 0,
            WmSubtileState      => 0,
            TileGetNextCritter  => 0,
            ArtSetBaseFidNum    => 0,
            TileSetCenter       => 0,
            AiGetChemUseValue   => 0,
            WmCarIsOutOfGas     => 0,
            MapTargetLoadArea   => 0,
        }
    } else {
        error!("unknown Metarule3 ID {}", id);
        0
    };

    ctx.prg.data_stack.push(Value::Int(r))?;

    if let Some(mr) = mr {
        log_a4r1!(ctx.prg, mr, v1, v2, v3, ctx.prg.data_stack.top().unwrap());
    } else {
        log_a4r1!(ctx.prg, id, v1, v2, v3, ctx.prg.data_stack.top().unwrap());
    }
    log_stub!(ctx.prg);

    Ok(())
}

pub fn override_map_start(ctx: Context) -> Result<()> {
    let direction = ctx.prg.data_stack.pop()?.into_int()?;
    let direction = Direction::from_i32(direction)
        .ok_or(Error::BadValue(BadValue::Content))?;

    let elevation = ctx.prg.data_stack.pop()?.into_int()? as u32;
    let y = ctx.prg.data_stack.pop()?.into_int()?;

    let world = &mut ctx.ext.world;

    let x = ctx.prg.data_stack.pop()?.into_int()?;
    let x = world.map_grid().hex().invert_x(x);

    let obj = world.dude_obj().unwrap();
    let pos = EPoint::new(elevation, (x, y));
    world.set_object_pos(obj, pos);
    world.objects_mut().get(obj).borrow_mut().direction = direction;

    world.map_grid_mut().center2(pos.point);

    log_a4!(ctx.prg, x, y, direction, elevation);

    Ok(())
}


pub fn party_member_obj(ctx: Context) -> Result<()> {
    let pid = ctx.prg.data_stack.pop()?.into_int()?;
    let r = Value::Object(None);
    ctx.prg.data_stack.push(r)?;
    log_a1r1!(ctx.prg, pid, ctx.prg.data_stack.top().unwrap());
    log_stub!(ctx.prg);
    Ok(())
}

pub fn reg_anim_animate_forever(ctx: Context) -> Result<()> {
    use crate::asset::frm::CritterAnim;
    use crate::game::sequence::frame_anim::*;

    let critter_anim = CritterAnim::from_i32(ctx.prg.data_stack.pop()?.into_int()?)
        .ok_or(Error::BadValue(BadValue::Content))?;
    let obj = ctx.prg.data_stack.pop()?.into_object()?;
    if let Some(obj) = obj {
        if !ctx.ext.has_running_sequence(obj) {
            let chain = ctx.prg.instr_state.sequences.entry(obj)
                .or_insert_with(|| Chain::endless().0);
            let seq = FrameAnim::new(obj, Some(critter_anim), AnimDirection::Forward, true);
            chain.push(seq);
        } else {
            debug!("reg_anim_animate_forever: object {:?} already has running sequence", obj);
        }
    }
    log_a2!(ctx.prg, obj, critter_anim);
    Ok(())
}

pub fn reg_anim_func(ctx: Context) -> Result<()> {
    let arg = ctx.prg.data_stack.pop()?;
    let op = RegAnimFuncOp::from_i32(ctx.prg.data_stack.pop()?.into_int()?)
        .ok_or(Error::BadValue(BadValue::Content))?;
    match op {
        RegAnimFuncOp::Begin => {
            let flags = arg.into_int()?;
            if !ctx.prg.instr_state.sequences.is_empty() {
                warn!("RegAnimFunc(Begin, ...): previous session wasn't ended properly with RegAnimFunc(End)");
            }
            ctx.prg.instr_state.sequences.clear();
            log_a2!(ctx.prg, op, flags);
        }
        RegAnimFuncOp::End => {
            for (objh, seq) in ctx.prg.instr_state.sequences.drain() {
                let (seq, cancel) = seq.cancellable();
                let mut obj = ctx.ext.world.objects().get(objh).borrow_mut();
                assert!(obj.sequence.is_none());
                obj.sequence = Some(cancel);
                ctx.ext.sequencer.start(seq);
            }

            log_a2!(ctx.prg, op, arg);
        }
        RegAnimFuncOp::Clear => {
            let obj = arg.into_object()?;
            if let Some(obj) = obj {
                if let Some(s) = ctx.ext.world.objects().get(obj).borrow_mut().sequence.take() {
                    s.cancel();
                }
            }
            log_a2!(ctx.prg, op, obj);
        }
    }
    Ok(())
}

pub fn set_light_level(ctx: Context) -> Result<()> {
    let v = cmp::min(cmp::max(ctx.prg.data_stack.pop()?.into_int()?, 0), 100) as u32;

    const MIN: u32 = 0x4000;
    const MID: u32 = 0xA000;
    const MAX: u32 = 0x10000;

    // TODO This probably should be fixed as follows:
    // if v < 50 { MIN + v * (MID - MIN) / 50 } else { MID + (v - 50) * (MAX - MID) / 50 }
    let light = if v < 50 {
        MIN + v * (MID - MIN) / 100
    } else if v == 50 {
        MID
    } else {
        MID + v * (MAX - MID) / 100
    };

    ctx.ext.world.ambient_light = light;

    log_a1!(ctx.prg, v);

    Ok(())
}

pub fn tile_contains_pid_obj(ctx: Context) -> Result<()> {
    let pid = ctx.prg.data_stack.pop()?.into_int()?;
    let pid = Pid::from_packed(pid as u32)
        .ok_or(Error::BadValue(BadValue::Content))?;

    let elevation = ctx.prg.data_stack.pop()?.into_int()? as u32;
    let tile_num = ctx.prg.data_stack.pop()?.into_int()?;

    let pos = ctx.ext.world.map_grid().hex().from_linear_inv(tile_num as u32)
        .elevated(elevation);

    let r = ctx.ext.world.objects().at(pos).iter()
        .any(|&obj| ctx.ext.world.objects().get(obj).borrow().pid == Some(pid));
    ctx.prg.data_stack.push(r.into())?;

    log_a3r1!(ctx.prg, tile_num, elevation, pid, ctx.prg.data_stack.top().unwrap());

    Ok(())
}

pub fn tile_num_in_direction(ctx: Context) -> Result<()> {
    let distance = ctx.prg.data_stack.pop()?.into_int()?;
    let direction = ctx.prg.data_stack.pop()?.into_int()?;
    let tile_num = ctx.prg.data_stack.pop()?.into_int()?;

    // FIXME clean up this, better validate
    use crate::graphics::geometry::hex::TileGrid;
    let hex = TileGrid::default();
    let p = hex.from_linear_inv(tile_num as u32);
    let r = hex.go(p, Direction::from_i32(direction).unwrap(), distance as u32)
        .map(|p| hex.to_linear_inv(p).unwrap() as i32)
        .unwrap_or(-1);
    ctx.prg.data_stack.push(Value::Int(r))?;

    log_a3r1!(ctx.prg, tile_num, direction, distance, ctx.prg.data_stack.top().unwrap());

    Ok(())
}
