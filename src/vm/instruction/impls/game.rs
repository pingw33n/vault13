use enum_map_derive::Enum;
use enum_primitive_derive::Primitive;
use log::*;
use num_traits::FromPrimitive;
use static_assertions::const_assert;
use std::cmp;
use std::convert::TryInto;

use super::*;
use crate::asset::{Perk, Stat, Trait};
use crate::asset::proto::ProtoId;
use crate::asset::script::ProgramId;
use crate::game::dialog::Dialog;
use crate::game::object::Object;
use crate::game::script::Sid;
use crate::game::world::floating_text;
use crate::graphics::{EPoint, Point};
use crate::graphics::color::*;
use crate::graphics::font::FontKey;
use crate::graphics::geometry::hex::Direction;
use crate::sequence::Sequence;
use crate::sequence::chain::Chain;
use crate::util::random::{random as rand};

fn pop_program_id(ctx: &mut Context) -> Result<ProgramId> {
    ctx.prg.data_stack.pop()?.into_int()?
        .try_into().ok()
        .and_then(ProgramId::new)
        .ok_or(Error::BadValue(BadValue::Content))
}

fn resolve_script_msg(msg: Value, program_id: ProgramId, ctx: &mut Context) -> Result<Rc<BString>> {
    Ok(match msg {
        Value::Int(msg_id) => {
            let msgs = ctx.ext.script_db.messages(program_id).unwrap();
            Rc::new(msgs.get(msg_id).unwrap().text.clone())
        }
        Value::String(msg) => {
            msg.resolve(ctx.prg.strings())?
        }
        _ => return Err(Error::BadValue(BadValue::Content)),
    })
}

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

pub fn add_obj_to_inven(ctx: Context) -> Result<()> {
    let item = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;
    let target = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;
    log_a2!(ctx.prg, target, item);
    log_stub!(ctx.prg);
    Ok(())
}

pub fn add_timer_event(ctx: Context) -> Result<()> {
    let info = ctx.prg.data_stack.pop()?.into_int()?;
    let time = ctx.prg.data_stack.pop()?.into_int()?;
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;

    log_a3!(ctx.prg, obj, time, info);
    log_stub!(ctx.prg);

    Ok(())
}

pub fn anim(ctx: Context) -> Result<()> {
    let direction = ctx.prg.data_stack.pop()?.into_int()?;
    let direction = Direction::from_i32(direction)
        .ok_or(Error::BadValue(BadValue::Content))?;

    let anim = ctx.prg.data_stack.pop()?.into_int()?;
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;

    log_a3!(ctx.prg, obj, anim, direction);
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
    let pid = ProtoId::from_packed(pid as u32)
        .ok_or(Error::BadValue(BadValue::Content))?;
    let proto = ctx.ext.proto_db.proto(pid)
        .map_err(|e| {
            error!("error loading proto {:?}: {:?}", pid, e);
            Error::BadValue(BadValue::Content)
        })?;

    // FIXME add proper impl
    let fid = proto.borrow().fid;
    let pos = ctx.ext.world.hex_grid().from_linear_inv(tile_num);
    let pos = pos.elevated(elevation);
    let obj = Object::new(fid, proto.into(), Some(pos));
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

pub fn critter_attempt_placement(ctx: Context) -> Result<()> {
    let elevation = ctx.prg.data_stack.pop()?.into_int()?;
    let tile_num = ctx.prg.data_stack.pop()?.into_int()?;
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;

    let r = 0;
    ctx.prg.data_stack.push(r.into())?;

    log_a3r1!(ctx.prg, obj, tile_num, elevation, r);
    log_stub!(ctx.prg);

    Ok(())
}

pub fn critter_inven_obj(ctx: Context) -> Result<()> {
    let query = ctx.prg.data_stack.pop()?.into_int()?;
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?;

    let r = 0;
    ctx.prg.data_stack.push(r.into())?;

    log_a2r1!(ctx.prg, obj, query, r);
    log_stub!(ctx.prg);
    Ok(())
}

pub fn cur_map_index(ctx: Context) -> Result<()> {
    let r = ctx.ext.map_id;
    ctx.prg.data_stack.push(r.try_into().unwrap())?;
    log_r1!(ctx.prg, r);
    Ok(())
}

pub fn destroy_object(ctx: Context) -> Result<()> {
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?;
    log_a1!(ctx.prg, obj);
    log_stub!(ctx.prg);
    Ok(())
}

pub fn display_msg(ctx: Context) -> Result<()> {
    use crate::ui::message_panel::MessagePanel;

    let msg = ctx.prg.data_stack.pop()?.into_string(ctx.prg.strings())?;

    // TODO dedup this and GameState::push_message()
    ctx.ext.ui.widget_mut::<MessagePanel>(ctx.ext.message_panel)
        .push_message(BString::concat(&[crate::asset::message::BULLET_STR, msg.as_bytes()]));

    log_a1!(ctx.prg, msg);
    Ok(())
}

pub fn dude_obj(ctx: Context) -> Result<()> {
    let obj = ctx.ext.world.dude_obj();
    ctx.prg.data_stack.push(Value::Object(obj))?;
    log_r1!(ctx.prg, obj);
    Ok(())
}

pub fn elevation(ctx: Context) -> Result<()> {
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;
    let pos = ctx.ext.world.objects().get(obj).pos;
    let r = pos.map(|p| p.elevation as i32).unwrap();
    ctx.prg.data_stack.push(r.into())?;
    log_a1r1!(ctx.prg, obj, r);
    Ok(())
}

pub fn end_dialogue(ctx: Context) -> Result<()> {
    ctx.ext.dialog.take().unwrap().hide(ctx.ext.ui, ctx.ext.world);
    log_!(ctx.prg);
    Ok(())
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Primitive)]
#[repr(u32)]
enum FloatingTextStyle {
    Sequential  = 0xffff_fffe,
    Warning     = 0xffff_ffff,
    Normal      = 0,
    Black       = 1,
    Red         = 2,
    Green       = 3,
    Blue        = 4,
    Purple      = 5,
    NearWhite   = 6,
    LightRed    = 7,
    Yellow      = 8,
    White       = 9,
    Gray        = 10,
    DarkGray    = 11,
    LightGray   = 12,
}

impl FloatingTextStyle {
    const SEQ_MIN: u32 = 1;
    const SEQ_MAX: u32 = 12;
}

const_assert!(FloatingTextStyle::SEQ_MIN <= FloatingTextStyle::SEQ_MAX);

pub fn float_msg(ctx: Context) -> Result<()> {
    let style = FloatingTextStyle::from_i32(ctx.prg.data_stack.pop()?.into_int()?);
    let msg = ctx.prg.data_stack.pop()?.into_string(ctx.prg.strings())?;
    let mut obj = ctx.prg.data_stack.pop()?.coerce_into_object()?;

    if obj.is_some() {
        use FloatingTextStyle::*;
        let style = style.unwrap_or(Normal);

        if style == Sequential {
            // In original it's true sequential, but random is easier to implement and
            // should provide the same features.
            FloatingTextStyle::from_i32(rand(-1, 12)).unwrap()
        } else {
            style
        };
        let mut font_key = FontKey::antialiased(1);
        let color = match style {
            Sequential => unreachable!(),
            Warning => {
                font_key = FontKey::antialiased(3);
                obj = None;
                RED
            }
            Normal | Yellow => Rgb15::from_packed(0x7feb),
            Black | Purple | Gray => BLACK,
            Red => RED,
            Green => GREEN,
            Blue => BLUE,
            NearWhite => Rgb15::from_packed(0x5294),
            LightRed => Rgb15::from_packed(0x7d4a),
            White => WHITE,
            DarkGray => Rgb15::from_packed(0x2108),
            LightGray => Rgb15::from_packed(0x3def),
        };
        ctx.ext.world.show_floating_text(obj, &*msg, floating_text::Options {
            font_key,
            color,
            outline_color: Some(BLACK),
        });
    }

    log_a3!(ctx.prg, obj, msg, style);

    Ok(())
}

#[test]
fn floating_text_style() {
    assert_eq!(FloatingTextStyle::from_i32(-2), Some(FloatingTextStyle::Sequential));
    assert_eq!(FloatingTextStyle::from_i32(-1), Some(FloatingTextStyle::Warning));

    for i in FloatingTextStyle::SEQ_MIN..=FloatingTextStyle::SEQ_MAX {
        assert!(FloatingTextStyle::from_u32(i).is_some());
    }
}

pub fn game_ticks(ctx: Context) -> Result<()> {
    let v = ctx.prg.data_stack.pop()?.into_int()?;

    let r = cmp::max(v, 0) * 10;
    ctx.prg.data_stack.push(r.into())?;

    log_a1r1!(ctx.prg, v, r);

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

pub fn get_critter_stat(ctx: Context) -> Result<()> {
    let stat = Stat::from_i32(ctx.prg.data_stack.pop()?.coerce_into_int()?)
        .ok_or(Error::BadValue(BadValue::Content))?;
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?;
    let r = 5;
    ctx.prg.data_stack.push(Value::Int(r))?;
    log_a2r1!(ctx.prg, obj, stat, r);
    log_stub!(ctx.prg);
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

pub fn giq_option(mut ctx: Context) -> Result<()> {
    // FIXME display reaction with Empathy perk.
    let reaction = ctx.prg.data_stack.pop()?.into_int()?;
    let proc = ctx.prg.data_stack.pop()?;
    let msg = ctx.prg.data_stack.pop()?;
    let program_id = pop_program_id(&mut ctx)?;
    let min_or_max_iq = ctx.prg.data_stack.pop()?.into_int()?;

    let msg = resolve_script_msg(msg, program_id, &mut ctx)?;

    let iq = 5_i32; // FIXME stat_level_(g_obj_dude, STAT_INT);
    let smooth_talker = 0; // FIXME perk_level_(g_obj_dude, PERK_smooth_talker);
    let iq = iq + smooth_talker;

    // FIXME proc can also be a string
    let proc_id = proc.into_int()?;

    assert!(ctx.ext.dialog.is_some());

    // If negative it defines upper bound, otherwise it's the lower bound.
    if min_or_max_iq < 0 && -iq >= min_or_max_iq || min_or_max_iq >= 0 && iq >= min_or_max_iq {
        let dialog = ctx.ext.dialog.as_mut().unwrap();
        dialog.add_option(ctx.ext.ui, &*msg, Some(proc_id as u32));
    }

    log_a5!(ctx.prg, min_or_max_iq, program_id, msg, proc_id, reaction);

    Ok(())
}

pub fn gsay_message(mut ctx: Context) -> Result<()> {
    // FIXME display reaction with Empathy perk.
    let reaction = ctx.prg.data_stack.pop()?.into_int()?;
    let msg = ctx.prg.data_stack.pop()?;
    let program_id = pop_program_id(&mut ctx)?;

    let reply = resolve_script_msg(msg, program_id, &mut ctx)?;
    let option = &ctx.ext.proto_db.messages().get(650).unwrap().text;

    assert!(ctx.ext.dialog.is_some());

    let dialog = ctx.ext.dialog.as_mut().unwrap();
    dialog.set_reply(ctx.ext.ui, &*reply);
    dialog.clear_options(ctx.ext.ui);
    dialog.add_option(ctx.ext.ui, option, None);

    log_a3!(ctx.prg, program_id, reply, reaction);

    Ok(())
}

pub fn gsay_end(ctx: Context) -> Result<Option<Suspend>> {
    let dialog = ctx.ext.dialog.as_mut().unwrap();
    assert!(!dialog.running);
    dialog.running = true;
    log_!(ctx.prg);
    Ok(Some(Suspend::GsayEnd))
}

pub fn gsay_start(ctx: Context) -> Result<()> {
    assert!(ctx.ext.dialog.is_some());
    log_!(ctx.prg);
    Ok(())
}

pub fn gsay_reply(mut ctx: Context) -> Result<()> {
    let reply = ctx.prg.data_stack.pop()?;
    let program_id = pop_program_id(&mut ctx)?;

    let reply_str = match reply {
        Value::Int(msg_id) => {
            let msgs = ctx.ext.script_db.messages(program_id).unwrap();
            Rc::new(msgs.get(msg_id).unwrap().text.clone())
        },
        Value::String(s) => s.resolve(ctx.prg.strings())?,
        _ => return Err(Error::BadValue(BadValue::Content)),
    };

    assert!(ctx.ext.dialog.is_some());
    let dialog = ctx.ext.dialog.as_mut().unwrap();
    dialog.set_reply(ctx.ext.ui, &*reply_str);
    dialog.clear_options(ctx.ext.ui);

    log_a2!(ctx.prg, reply_str, program_id);

    Ok(())
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Primitive)]
enum TraitFamilyKind {
    Perk = 0,
    Object = 1,
    Trait = 2,
}

#[derive(Clone, Copy, Debug)]
enum TraitFamily {
    Perk(Perk),
    PerkUnknown(i32),
    Object(ObjectTrait),
    ObjectUnknown(i32),
    Trait(Trait),
    TraitUnknown(i32),
    Unknown(i32),
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq, Primitive)]
enum ObjectTrait {
    AiPacket = 5,
    TeamNum = 6,
    Direction = 10,
    IsTurnedOff = 666,
    ItemTotalWeight = 669,
}

pub fn has_trait(ctx: Context) -> Result<()> {
    let kind = ctx.prg.data_stack.pop()?.into_int()?;
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?;
    let family_kind = ctx.prg.data_stack.pop()?.coerce_into_int()?;
    let family = match TraitFamilyKind::from_i32(family_kind) {
        Some(TraitFamilyKind::Perk) => Perk::from_i32(kind)
            .map(TraitFamily::Perk)
            .unwrap_or(TraitFamily::PerkUnknown(kind)),
        Some(TraitFamilyKind::Object) => ObjectTrait::from_i32(kind)
            .map(TraitFamily::Object)
            .unwrap_or(TraitFamily::ObjectUnknown(kind)),
        Some(TraitFamilyKind::Trait) => Trait::from_i32(kind)
            .map(TraitFamily::Trait)
            .unwrap_or(TraitFamily::TraitUnknown(kind)),
        None => TraitFamily::Unknown(family_kind),
    };
    let r = 0;
    ctx.prg.data_stack.push(Value::Int(r))?;
    log_a2r1!(ctx.prg, family, obj, r);
    log_stub!(ctx.prg);
    Ok(())
}

pub fn message_str(mut ctx: Context) -> Result<()> {
    let msg_id = ctx.prg.data_stack.pop()?.into_int()?;
    let program_id = pop_program_id(&mut ctx)?;

    let msgs = ctx.ext.script_db.messages(program_id).unwrap();
    let msg = Rc::new(msgs.get(msg_id).unwrap().text.clone());

    ctx.prg.data_stack.push(msg.clone().into())?;
    log_a2r1!(ctx.prg, program_id, msg_id, msg);

    Ok(())
}

pub fn metarule(ctx: Context) -> Result<()> {
    let arg = ctx.prg.data_stack.pop()?;
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
        log_a2r1!(ctx.prg, mr, arg, ctx.prg.data_stack.top().unwrap());
    } else {
        log_a2r1!(ctx.prg, id, arg, ctx.prg.data_stack.top().unwrap());
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

pub fn move_obj_inven_to_obj(ctx: Context) -> Result<()> {
    let dst = ctx.prg.data_stack.pop()?.coerce_into_object()?;
    let src = ctx.prg.data_stack.pop()?.coerce_into_object()?;

    if src.is_none() {
        log_error!(ctx.prg, "src object is null");
    }
    if dst.is_none() {
        log_error!(ctx.prg, "dst object is null");
    }

    log_a2!(ctx.prg, src, dst);
    log_stub!(ctx.prg);

    Ok(())
}

pub fn move_to(ctx: Context) -> Result<()> {
    let elevation = ctx.prg.data_stack.pop()?.coerce_into_int();
    let tile_num = ctx.prg.data_stack.pop()?.coerce_into_int();
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;

    let r = 0;
    ctx.prg.data_stack.push(r.into())?;

    log_a3r1!(ctx.prg, obj, tile_num, elevation, r);
    log_stub!(ctx.prg);

    Ok(())
}

pub fn obj_art_fid(ctx: Context) -> Result<()> {
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;

    let r = ctx.ext.world.objects().get(obj).fid;

    ctx.prg.data_stack.push(Value::Int(r.packed() as i32))?;
    log_a1r1!(ctx.prg, obj, r);

    Ok(())
}

pub fn obj_can_see_obj(ctx: Context) -> Result<()> {
    let obj2 = ctx.prg.data_stack.pop()?.coerce_into_object()?;
    let obj1 = ctx.prg.data_stack.pop()?.coerce_into_object()?;

    if obj1.is_none() || obj2.is_none() {
        log_error!(ctx.prg, "obj1 or obj2 is null");
    }

    let r = false;
    ctx.prg.data_stack.push(r.into())?;

    log_a2r1!(ctx.prg, obj1, obj2, r);
    log_stub!(ctx.prg);
    Ok(())
}

pub fn obj_is_carrying_obj_pid(ctx: Context) -> Result<()> {
    let pid = ctx.prg.data_stack.pop()?.into_int()?;
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;

    let r = 0;
    ctx.prg.data_stack.push(r.into())?;

    log_a2r1!(ctx.prg, obj, pid, r);
    log_stub!(ctx.prg);

    Ok(())
}

pub fn obj_lock(ctx: Context) -> Result<()> {
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;

    log_a1!(ctx.prg, obj);
    log_stub!(ctx.prg);

    Ok(())
}

pub fn obj_name(ctx: Context) -> Result<()> {
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;

    let r = Rc::new(ctx.ext.world.object_name(obj).unwrap_or_default());

    ctx.prg.data_stack.push(r.clone().into())?;
    log_a1r1!(ctx.prg, obj, r);

    Ok(())
}

pub fn obj_on_screen(ctx: Context) -> Result<()> {
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;

    let r = ctx.ext.world.is_object_in_camera(obj);
    ctx.prg.data_stack.push(r.into())?;

    log_a1r1!(ctx.prg, obj, r);

    Ok(())
}

pub fn obj_pid(ctx: Context) -> Result<()> {
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?;

    if obj.is_none() {
        log_error!(ctx.prg, "object is null");
    }

    let r = obj
        .and_then(|obj| ctx.ext.world.objects().get(obj).proto_id())
        .map(|pid| pid.pack() as i32)
        .unwrap_or(-1);
    ctx.prg.data_stack.push(r.into())?;

    log_a1r1!(ctx.prg, obj, r);
    Ok(())
}

pub fn obj_unlock(ctx: Context) -> Result<()> {
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;

    log_a1!(ctx.prg, obj);
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
    let x = world.hex_grid().invert_x(x);

    let obj = world.dude_obj().unwrap();
    let pos = EPoint::new(elevation, Point::new(x, y));
    world.set_object_pos(obj, pos);
    world.objects_mut().get_mut(obj).direction = direction;

    world.camera_mut().look_at(pos.point);

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

pub fn random(ctx: Context) -> Result<()> {
    let to_incl = ctx.prg.data_stack.pop()?.into_int()?;
    let from_incl = ctx.prg.data_stack.pop()?.into_int()?;

    // TODO check if vcr_status() == 2 condition in orginal is important.

    let r = rand(from_incl, to_incl);
    ctx.prg.data_stack.push(r.into())?;

    log_a2r1!(ctx.prg, from_incl, to_incl, r);

    Ok(())
}

pub fn reg_anim_animate_forever(ctx: Context) -> Result<()> {
    use crate::asset::CritterAnim;
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
                let mut obj = ctx.ext.world.objects().get_mut(objh);
                assert!(obj.sequence.is_none());
                obj.sequence = Some(cancel);
                ctx.ext.sequencer.start(seq);
            }

            log_a2!(ctx.prg, op, arg);
        }
        RegAnimFuncOp::Clear => {
            let obj = arg.into_object()?;
            if let Some(obj) = obj {
                if let Some(s) = ctx.ext.world.objects().get_mut(obj).sequence.take() {
                    s.cancel();
                }
            }
            log_a2!(ctx.prg, op, obj);
        }
    }
    Ok(())
}

pub fn rm_timer_event(ctx: Context) -> Result<()> {
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;

    log_a1!(ctx.prg, obj);
    log_stub!(ctx.prg);

    Ok(())
}

pub fn set_light_level(ctx: Context) -> Result<()> {
    let v = cmp::min(cmp::max(ctx.prg.data_stack.pop()?.into_int()?, 0), 100) as u32;

    const MIN: u32 = 0x4000;
    const MID: u32 = 0xA000;
    const MAX: u32 = 0x10000;

    // TODO This probably should be fixed as follows:
    // if v < 50 { MIN + v * (MID - MIN) / 50 } else { MID + (v - 50) * (MAX - MID) / 50 }
    let light = match v {
        0..=49 => MIN + v * (MID - MIN) / 100,
        50 => MID,
        _ => MID + v * (MAX - MID) / 100,
    };

    ctx.ext.world.ambient_light = light;

    log_a1!(ctx.prg, v);

    Ok(())
}

pub fn set_obj_visibility(ctx: Context) -> Result<()> {
    let visible = ctx.prg.data_stack.pop()?.into_bool()?;
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;

    log_a2!(ctx.prg, obj, visible);
    log_stub!(ctx.prg);

    Ok(())
}

pub fn start_gdialog(mut ctx: Context) -> Result<()> {
    let background = ctx.prg.data_stack.pop()?.into_int()?;
    let head_id = ctx.prg.data_stack.pop()?.into_int()?;
    let reaction = ctx.prg.data_stack.pop()?.into_int()?;
    let objh = ctx.prg.data_stack.pop()?.coerce_into_object()?.unwrap();
    let program_id = pop_program_id(&mut ctx)?;

    // TODO disallow in combat state
    // TODO handle head_id
    // TODO check for can_talk() (or can_talk_now()?)

    assert!(ctx.ext.dialog.is_none());
    *ctx.ext.dialog = Some(Dialog::show(ctx.ext.ui, ctx.ext.world, objh));

    log_a5!(ctx.prg, program_id, objh, reaction, head_id, background);

    Ok(())
}

pub fn tile_contains_pid_obj(ctx: Context) -> Result<()> {
    let pid = ctx.prg.data_stack.pop()?.into_int()?;
    let pid = ProtoId::from_packed(pid as u32)
        .ok_or(Error::BadValue(BadValue::Content))?;

    let elevation = ctx.prg.data_stack.pop()?.into_int()? as u32;
    let tile_num = ctx.prg.data_stack.pop()?.into_int()?;

    let pos = ctx.ext.world.hex_grid().from_linear_inv(tile_num as u32)
        .elevated(elevation);

    let r = ctx.ext.world.objects().at(pos).iter()
        .any(|&obj| ctx.ext.world.objects().get(obj).proto_id() == Some(pid));
    ctx.prg.data_stack.push(r.into())?;

    log_a3r1!(ctx.prg, tile_num, elevation, pid, ctx.prg.data_stack.top().unwrap());

    Ok(())
}

pub fn tile_in_tile_rect(ctx: Context) -> Result<()> {
    let tile_num = ctx.prg.data_stack.pop()?.into_int()?;
    let right = ctx.prg.data_stack.pop()?.into_int()?;
    let bottom = ctx.prg.data_stack.pop()?.into_int()?;
    let top = ctx.prg.data_stack.pop()?.into_int()?;
    let left = ctx.prg.data_stack.pop()?.into_int()?;

    let r = 0;
    ctx.prg.data_stack.push(r.into())?;

    log_a5r1!(ctx.prg, left, top, bottom, right, tile_num, r);
    log_stub!(ctx.prg);

    Ok(())
}

pub fn tile_num(ctx: Context) -> Result<()> {
    let obj = ctx.prg.data_stack.pop()?.coerce_into_object()?
        .ok_or(Error::BadValue(BadValue::Content))?;
    let pos = ctx.ext.world.objects().get(obj).pos;
    let r = pos.map(|p| {
        // FIXME clean up this
        use crate::graphics::geometry::hex::TileGrid;
        let hex = TileGrid::default();
        hex.to_linear_inv(p.point).unwrap() as i32
    }).unwrap();
    ctx.prg.data_stack.push(r.into())?;
    log_a1r1!(ctx.prg, obj, r);
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
