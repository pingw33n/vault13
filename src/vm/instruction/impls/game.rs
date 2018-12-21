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

pub fn set_light_level(ctx: Context) -> Result<()> {
    let v = ctx.vm_state.data_stack.pop()?.into_int()?;
    log_a1!(ctx.vm_state, v);
    log_stub!(ctx.vm_state);
    Ok(())
}

pub fn metarule(ctx: Context) -> Result<()> {
    let value = ctx.vm_state.data_stack.pop()?.into_int()?;
    let id = ctx.vm_state.data_stack.pop()?.into_int()?;

    use self::Metarule::*;
    let r = if let Some(mr) = Metarule::from_i32(id) {
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

    ctx.vm_state.data_stack.push(Value::Int(r))?;

    log_a2r1!(ctx.vm_state, id, value, ctx.vm_state.data_stack.top().unwrap());
    log_stub!(ctx.vm_state);

    Ok(())
}

pub fn metarule3(ctx: Context) -> Result<()> {
    let v2 = ctx.vm_state.data_stack.pop()?.into_int()?;
    let v1 = ctx.vm_state.data_stack.pop()?.into_int()?;
    let id = ctx.vm_state.data_stack.pop()?.into_int()?;

    use self::Metarule3::*;
    let r = if let Some(mr) = Metarule3::from_i32(id) {
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

    ctx.vm_state.data_stack.push(Value::Int(r))?;

    log_a3r1!(ctx.vm_state, id, v1, v2, ctx.vm_state.data_stack.top().unwrap());
    log_stub!(ctx.vm_state);

    Ok(())
}