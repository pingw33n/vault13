use enum_map_derive::Enum;
use enum_primitive_derive::Primitive;
use log::*;
use num_traits::FromPrimitive;

use super::*;

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

pub fn destroy_object(ctx: Context) -> Result<()> {
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?;
    log_a1!(ctx.prg, obj);
    log_stub!(ctx.prg);
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

pub fn party_member_obj(ctx: Context) -> Result<()> {
    let pid = ctx.prg.data_stack.pop()?.into_int()?;
    let r = Value::Object(None);
    ctx.prg.data_stack.push(r)?;
    log_a1r1!(ctx.prg, pid, ctx.prg.data_stack.top().unwrap());
    log_stub!(ctx.prg);
    Ok(())
}

pub fn set_light_level(ctx: Context) -> Result<()> {
    let v = ctx.prg.data_stack.pop()?.into_int()?;
    log_a1!(ctx.prg, v);
    log_stub!(ctx.prg);
    Ok(())
}

pub fn tile_contains_pid_obj(ctx: Context) -> Result<()> {
    let pid = ctx.prg.data_stack.pop()?.into_int()?;
    let elevation = ctx.prg.data_stack.pop()?.into_int()?;
    let tile_num = ctx.prg.data_stack.pop()?.into_int()?;

    let r = false;
    ctx.prg.data_stack.push(r.into())?;

    log_a3r1!(ctx.prg, tile_num, elevation, pid, ctx.prg.data_stack.top().unwrap());
    log_stub!(ctx.prg);

    Ok(())
}

pub fn tile_num_in_direction(ctx: Context) -> Result<()> {
    let distance = ctx.prg.data_stack.pop()?.into_int()?;
    let direction = ctx.prg.data_stack.pop()?.into_int()?;
    let tile_num = ctx.prg.data_stack.pop()?.into_int()?;

    // FIXME clean up this, better validate
    use crate::graphics::geometry::hex::{Direction, TileGrid};
    let hex = TileGrid::default();
    let p = hex.from_linear_inv(tile_num as u32);
    let r = hex.go(p, Direction::from_i32(direction).unwrap(), distance as u32)
        .map(|p| hex.to_linear_inv(p).unwrap() as i32)
        .unwrap_or(-1);
    ctx.prg.data_stack.push(Value::Int(r))?;

    log_a3r1!(ctx.prg, tile_num, direction, distance, ctx.prg.data_stack.top().unwrap());

    Ok(())
}
