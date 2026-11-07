//! Small widgets shared by the views, drawn from the active palette.

use eframe::egui::{self, Button, Response, RichText, Role, Vec2, WidgetInfo};
use egui_material_icons::MaterialIcon;

use crate::theme;

/// How a toolbar button reads: the one main action is filled with the accent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emphasis {
    Primary,
    Normal,
}

/// A button showing only an icon. `label` is its name: shown as a tooltip and given to
/// screen readers (and tests), which would otherwise read the icon's private codepoint.
pub fn icon_button(
    ui: &mut egui::Ui,
    icon: MaterialIcon,
    label: &str,
    emphasis: Emphasis,
) -> Response {
    let palette = theme::palette(ui.ctx());
    let enabled = ui.is_enabled();
    let text = icon.rich_text().size(18.0);
    let button = if emphasis == Emphasis::Primary && enabled {
        Button::new(text.color(palette.on_accent)).fill(palette.accent)
    } else {
        Button::new(text)
    };
    let response = ui
        .add(button.min_size(Vec2::new(32.0, 28.0)))
        .on_hover_text(label);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, label));
    response
}

/// A small caption over a value, for live readouts such as the tick.
pub fn readout(ui: &mut egui::Ui, caption: &str, value: &str) {
    let palette = theme::palette(ui.ctx());
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(RichText::new(caption).small().color(palette.secondary));
        ui.label(
            RichText::new(value)
                .family(egui::FontFamily::Name(theme::SEMIBOLD_FAMILY.into()))
                .size(14.0)
                .color(palette.text),
        );
    });
}
