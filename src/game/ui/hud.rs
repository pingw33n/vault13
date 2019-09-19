use crate::asset::frame::FrameId;
use crate::graphics::Rect;
use crate::graphics::color::GREEN;
use crate::graphics::font::FontKey;
use crate::graphics::sprite::Sprite;
use crate::ui::*;
use crate::ui::button::Button;
use crate::ui::message_panel::{MessagePanel, Anchor};

pub fn create(ui: &mut Ui) -> Handle {
    let main_hud = ui.new_window(Rect::with_size(0, 379, 640, 100), Some(Sprite::new(FrameId::IFACE)));

    // Message panel.
    let mut mp = MessagePanel::new(ui.fonts().clone(), FontKey::antialiased(1), GREEN);
    mp.set_skew(1);
    mp.set_capacity(Some(100));
    mp.set_anchor(Anchor::Bottom);
    let message_panel = ui.new_widget(main_hud, Rect::with_size(23, 26, 166, 65), None, None, mp);

    // Inventory button.
    // Original location is a bit off, at y=41.
    ui.new_widget(main_hud, Rect::with_size(211, 40, 32, 21), None, None,
        Button::new(FrameId::INVENTORY_BUTTON_UP, FrameId::INVENTORY_BUTTON_DOWN));

    // Options button.
    ui.new_widget(main_hud, Rect::with_size(210, 62, 34, 34), None, None,
        Button::new(FrameId::OPTIONS_BUTTON_UP, FrameId::OPTIONS_BUTTON_DOWN));

    // Single/burst switch button.
    ui.new_widget(main_hud, Rect::with_size(218, 6, 22, 21), None, None,
        Button::new(FrameId::BIG_RED_BUTTON_UP, FrameId::BIG_RED_BUTTON_DOWN));

    // Skilldex button.
    ui.new_widget(main_hud, Rect::with_size(523, 6, 22, 21), None, None,
        Button::new(FrameId::BIG_RED_BUTTON_UP, FrameId::BIG_RED_BUTTON_DOWN));

    // MAP button.
    ui.new_widget(main_hud, Rect::with_size(526, 40, 41, 19), None, None,
        Button::new(FrameId::MAP_BUTTON_UP, FrameId::MAP_BUTTON_DOWN));

    // CHA button.
    ui.new_widget(main_hud, Rect::with_size(526, 59, 41, 19), None, None,
        Button::new(FrameId::CHARACTER_BUTTON_UP, FrameId::CHARACTER_BUTTON_DOWN));

    // PIP button.
    ui.new_widget(main_hud, Rect::with_size(526, 78, 41, 19), None, None,
        Button::new(FrameId::PIP_BUTTON_UP, FrameId::PIP_BUTTON_DOWN));

    // Attack button.
    // FIXME this should be a custom button with overlay text images.
    ui.new_widget(main_hud, Rect::with_size(267, 26, 188, 67), None, None,
        Button::new(FrameId::SINGLE_ATTACK_BUTTON_UP, FrameId::SINGLE_ATTACK_BUTTON_DOWN));

    message_panel
}