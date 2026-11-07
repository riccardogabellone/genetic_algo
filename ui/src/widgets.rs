//! Small widgets shared by the views, drawn from the active palette.

use eframe::egui::{
    self, Button, Color32, CornerRadius, Margin, Rect, Response, RichText, Role, Stroke, Vec2,
    WidgetInfo,
};
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

/// An on/off switch with its label on the left, for rules that are either on or off. The
/// label is part of the switch: clicking it toggles too, and screen readers read it.
pub fn switch(ui: &mut egui::Ui, on: &mut bool, label: &str) -> Response {
    let palette = theme::palette(ui.ctx());
    let height = ui.spacing().interact_size.y * 0.75;
    let track = Vec2::new(height * 1.8, height);
    let gap = ui.spacing().item_spacing.x;
    let galley = egui::WidgetText::from(label).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Body,
    );
    let size = Vec2::new(
        galley.size().x + gap + track.x,
        galley.size().y.max(ui.spacing().interact_size.y),
    );
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let enabled = ui.is_enabled();
    let state = *on;
    response.widget_info(|| WidgetInfo::selected(Role::CheckBox, enabled, state, label));

    if ui.is_rect_visible(rect) {
        let progress = ui.ctx().animate_bool_responsive(response.id, *on);
        let text_color = if enabled { palette.text } else { palette.dim };
        let text_pos = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(text_pos, galley, text_color);

        let track_rect = Rect::from_min_size(
            egui::pos2(rect.right() - track.x, rect.center().y - track.y / 2.0),
            track,
        );
        let off_fill = if response.hovered() {
            palette.surface_hover
        } else {
            palette.surface_active
        };
        let fill = lerp_color(off_fill, palette.accent, progress);
        let radius = track.y / 2.0;
        ui.painter().rect_filled(track_rect, radius, fill);
        let knob_x = egui::lerp(
            (track_rect.left() + radius)..=(track_rect.right() - radius),
            progress,
        );
        let knob = if enabled { palette.text } else { palette.dim };
        let knob = lerp_color(knob, palette.on_accent, progress);
        ui.painter().circle_filled(
            egui::pos2(knob_x, track_rect.center().y),
            radius * 0.7,
            knob,
        );
    }
    response
}

fn lerp_color(from: Color32, to: Color32, amount: f32) -> Color32 {
    let channel = |a: u8, b: u8| egui::lerp(f32::from(a)..=f32::from(b), amount).round() as u8;
    Color32::from_rgb(
        channel(from.r(), to.r()),
        channel(from.g(), to.g()),
        channel(from.b(), to.b()),
    )
}

/// A group of related settings: a titled frame. With a switch in the header, the switch
/// turns the group's rule on and off, and the settings show only while it is on.
/// Returns whether the switch changed.
pub fn card(
    ui: &mut egui::Ui,
    title: &str,
    switch: Option<(&mut bool, &str)>,
    body: impl FnOnce(&mut egui::Ui),
) -> bool {
    let palette = theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let mut changed = false;
            let mut open = true;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title)
                        .family(egui::FontFamily::Name(theme::SEMIBOLD_FAMILY.into()))
                        .color(palette.text),
                );
                if let Some((on, label)) = switch {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        changed = self::switch(ui, on, label).changed();
                    });
                    open = *on;
                }
            });
            if open {
                body(ui);
            }
            changed
        })
        .inner
}
