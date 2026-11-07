//! Colours, fonts and spacing. Every colour the UI's chrome draws comes from a [`Palette`];
//! views ask for the active one with [`palette`] instead of naming colours themselves. The
//! map is the exception: its cells use the core's palette, shared with the terminal and
//! GIF output, and its marks (leader ring, selection) are fixed, since the map is dark in
//! both themes.
//!
//! The colour roles, the way they map onto egui's visuals and the text-rendering
//! adjustments follow fastframe (https://fastframe.dev, MIT): fastframe-theme for the
//! roles and the derivation recipe, ZapFast's `theme.rs` for the mapping, fastframe-text
//! for the text rendering. See `ui/DESIGN.md`.

use std::collections::BTreeMap;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Shadow,
    Stroke, TextStyle, Theme, ThemePreference, Visuals,
    epaint::{FontColorTransferFunction, text::VariationCoords},
    style::{Selection, TextCursorStyle, WidgetVisuals, Widgets},
};

/// Inter with fixed-width digits; see `assets/fonts/README.md`.
const INTER: &[u8] = include_bytes!("../../assets/fonts/InterVariable.ttf");
const REGULAR: f32 = 400.0;
const SEMIBOLD: f32 = 600.0;
/// The font family for headings and stat values.
pub const SEMIBOLD_FAMILY: &str = "inter-semibold";

const LIGHT_BACKGROUND: Color32 = Color32::from_rgb(246, 248, 252);
const LIGHT_TEXT: Color32 = Color32::from_rgb(24, 32, 48);

/// Every colour role the UI uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// Behind the map: the darkest (or lightest) surface.
    pub window: Color32,
    /// Side panels and the toolbar.
    pub panel: Color32,
    /// Cards and resting widgets.
    pub surface: Color32,
    pub surface_hover: Color32,
    pub surface_active: Color32,
    pub outline: Color32,
    pub text: Color32,
    /// Labels and descriptions next to values.
    pub secondary: Color32,
    /// Hints and disabled text.
    pub dim: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub warning: Color32,
    /// Smartness from 0 to 100 as a colour: the logo's gradient, adjusted for contrast.
    pub smartness: [Color32; 4],
    /// The map stays dark in both themes, so its colours read the same way.
    pub map_background: Color32,
}

/// `from` blended towards `to` by `amount` (0 to 1), per channel.
pub(crate) const fn mix(from: Color32, to: Color32, amount: f32) -> Color32 {
    const fn channel(from: u8, to: u8, amount: f32) -> u8 {
        (from as f32 + (to as f32 - from as f32) * amount + 0.5) as u8
    }
    Color32::from_rgb(
        channel(from.r(), to.r(), amount),
        channel(from.g(), to.g(), amount),
        channel(from.b(), to.b(), amount),
    )
}

impl Palette {
    /// The fastframe-theme recipe: every neutral is the background blended towards the
    /// text colour, so a palette needs only a background, a text colour and an accent.
    const fn derive(background: Color32, text: Color32, accent: Color32) -> Self {
        Self {
            window: background,
            panel: mix(background, text, 0.03),
            surface: mix(background, text, 0.08),
            surface_hover: mix(background, text, 0.12),
            surface_active: mix(background, text, 0.18),
            outline: mix(background, text, 0.20),
            text,
            secondary: mix(background, text, 0.70),
            dim: mix(background, text, 0.50),
            accent,
            accent_hover: mix(accent, text, 0.15),
            on_accent: background,
            danger: Color32::PLACEHOLDER,
            warning: Color32::PLACEHOLDER,
            smartness: [Color32::PLACEHOLDER; 4],
            map_background: Color32::from_rgb(10, 15, 28),
        }
    }

    /// Navy from the logo, with its teal as the accent.
    pub const DARK: Self = Self {
        danger: Color32::from_rgb(255, 107, 107),
        warning: Color32::from_rgb(255, 210, 31),
        smartness: [
            Color32::from_rgb(0, 209, 224),
            Color32::from_rgb(131, 224, 136),
            Color32::from_rgb(255, 210, 31),
            Color32::from_rgb(254, 154, 13),
        ],
        ..Self::derive(
            Color32::from_rgb(12, 18, 34),
            Color32::from_rgb(227, 232, 242),
            Color32::from_rgb(34, 199, 207),
        )
    };

    /// The same hues, darkened where they would wash out on white.
    pub const LIGHT: Self = Self {
        // The recipe's 70% falls short of 4.5:1 on pressed widgets in the light theme.
        secondary: mix(LIGHT_BACKGROUND, LIGHT_TEXT, 0.76),
        danger: Color32::from_rgb(200, 52, 52),
        warning: Color32::from_rgb(166, 110, 0),
        smartness: [
            Color32::from_rgb(0, 140, 158),
            Color32::from_rgb(46, 150, 72),
            Color32::from_rgb(176, 124, 0),
            Color32::from_rgb(220, 104, 0),
        ],
        ..Self::derive(LIGHT_BACKGROUND, LIGHT_TEXT, Color32::from_rgb(0, 128, 140))
    };

    /// The colour for a smartness value, interpolated along [`Self::smartness`].
    pub fn smartness_color(&self, smartness: f32) -> Color32 {
        let stops = self.smartness.len() - 1;
        let position = (smartness / 100.0).clamp(0.0, 1.0) * stops as f32;
        let index = (position.floor() as usize).min(stops - 1);
        mix(
            self.smartness[index],
            self.smartness[index + 1],
            position - index as f32,
        )
    }
}

/// The palette of the theme the window is showing now.
pub fn palette(context: &egui::Context) -> &'static Palette {
    match context.theme() {
        Theme::Dark => &Palette::DARK,
        Theme::Light => &Palette::LIGHT,
    }
}

/// Fonts, spacing and both themes. Call once, before the first frame.
pub fn install(context: &egui::Context) {
    context.set_fonts(fonts());
    // After `set_fonts`, which would otherwise replace it.
    egui_material_icons::initialize(context);
    for (theme, palette) in [
        (Theme::Dark, &Palette::DARK),
        (Theme::Light, &Palette::LIGHT),
    ] {
        context.style_mut_of(theme, |style| {
            style.visuals = visuals(palette, theme);
            style.text_styles = text_styles();
            let spacing = &mut style.spacing;
            spacing.item_spacing = egui::vec2(8.0, 6.0);
            spacing.button_padding = egui::vec2(10.0, 4.0);
            spacing.interact_size = egui::vec2(40.0, 24.0);
            spacing.window_margin = Margin::same(12);
            spacing.menu_margin = Margin::same(8);
        });
    }
    context.set_theme(ThemePreference::System);
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let mut weighted = |name: &str, weight: f32| {
        let mut data = FontData::from_static(INTER);
        data.tweak.coords = VariationCoords::new([(b"wght", weight)]);
        fonts.font_data.insert(name.to_owned(), data.into());
    };
    weighted("inter", REGULAR);
    weighted(SEMIBOLD_FAMILY, SEMIBOLD);

    // egui's own fonts stay behind Inter as fallbacks (symbols, emoji).
    let fallbacks = fonts.families[&FontFamily::Proportional].clone();
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .expect("egui defines a proportional family")
        .insert(0, "inter".to_owned());
    fonts.families.insert(
        FontFamily::Name(SEMIBOLD_FAMILY.into()),
        std::iter::once(SEMIBOLD_FAMILY.to_owned())
            .chain(fallbacks)
            .collect(),
    );
    fonts
}

fn text_styles() -> BTreeMap<TextStyle, FontId> {
    let semibold = FontFamily::Name(SEMIBOLD_FAMILY.into());
    BTreeMap::from([
        (TextStyle::Small, FontId::proportional(11.5)),
        (TextStyle::Body, FontId::proportional(13.5)),
        (TextStyle::Button, FontId::proportional(13.5)),
        (TextStyle::Monospace, FontId::monospace(12.5)),
        (TextStyle::Heading, FontId::new(15.0, semibold)),
    ])
}

fn visuals(palette: &Palette, theme: Theme) -> Visuals {
    let mut visuals = match theme {
        Theme::Dark => Visuals::dark(),
        Theme::Light => Visuals::light(),
    };
    let widget = |fill: Color32, stroke: Stroke| WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: stroke,
        corner_radius: CornerRadius::same(6),
        fg_stroke: Stroke::new(1.0, palette.text),
        expansion: 0.0,
    };
    // Resting and hovered widgets differ by tint, not by a border.
    visuals.widgets = Widgets {
        noninteractive: widget(palette.panel, Stroke::new(1.0, palette.outline)),
        inactive: widget(palette.surface, Stroke::NONE),
        hovered: widget(palette.surface_hover, Stroke::NONE),
        active: widget(palette.surface_active, Stroke::new(1.0, palette.accent)),
        open: widget(palette.surface_hover, Stroke::NONE),
    };
    visuals.selection = Selection {
        bg_fill: palette.accent.gamma_multiply(0.35),
        stroke: Stroke::new(1.0, palette.accent),
    };
    visuals.weak_text_color = Some(palette.secondary);
    visuals.hyperlink_color = palette.accent;
    visuals.panel_fill = palette.panel;
    visuals.window_fill = palette.surface;
    visuals.window_stroke = Stroke::new(1.0, palette.outline);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.menu_corner_radius = CornerRadius::same(8);
    let shadow = Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(if theme == Theme::Dark { 110 } else { 40 }),
    };
    visuals.window_shadow = shadow;
    visuals.popup_shadow = shadow;
    visuals.extreme_bg_color = palette.window;
    visuals.faint_bg_color = palette.surface;
    visuals.code_bg_color = palette.surface;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.danger;
    visuals.text_cursor = TextCursorStyle {
        stroke: Stroke::new(2.0, palette.accent),
        ..visuals.text_cursor
    };
    visuals.slider_trailing_fill = true;
    adjust_text_rendering(&mut visuals);
    visuals
}

/// Matches the desktop's text rendering, after fastframe-text, which measured egui against
/// each platform's native text. On Linux, egui's dark-theme coverage curve draws text
/// 11-18% heavier than GTK, while linear coverage comes within 2%. macOS renders unhinted.
/// Windows matches egui's defaults.
fn adjust_text_rendering(visuals: &mut Visuals) {
    if cfg!(target_os = "linux") {
        visuals.text_options.color_transfer_function = FontColorTransferFunction::Off;
    }
    if cfg!(target_os = "macos") {
        visuals.text_options.font_hinting = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG 2 contrast ratio between two opaque colours.
    fn contrast(a: Color32, b: Color32) -> f32 {
        let luminance = |color: Color32| {
            let linear = |channel: u8| {
                let value = f32::from(channel) / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
        };
        let (light, dark) = {
            let (x, y) = (luminance(a), luminance(b));
            (x.max(y), x.min(y))
        };
        (light + 0.05) / (dark + 0.05)
    }

    #[test]
    fn text_is_readable_on_every_surface_in_both_themes() {
        for palette in [&Palette::DARK, &Palette::LIGHT] {
            for surface in [
                palette.window,
                palette.panel,
                palette.surface,
                palette.surface_active,
            ] {
                assert!(contrast(palette.text, surface) >= 7.0, "{palette:?}");
                assert!(contrast(palette.secondary, surface) >= 4.5, "{palette:?}");
            }
            assert!(
                contrast(palette.accent, palette.panel) >= 3.0,
                "{palette:?}"
            );
            assert!(
                contrast(palette.warning, palette.panel) >= 3.0,
                "{palette:?}"
            );
            assert!(
                contrast(palette.danger, palette.panel) >= 3.0,
                "{palette:?}"
            );
            for color in palette.smartness {
                assert!(
                    contrast(color, palette.window) >= 3.0,
                    "{color:?} in {palette:?}"
                );
            }
        }
    }

    #[test]
    fn the_smartness_gradient_runs_from_its_first_to_its_last_stop() {
        let palette = &Palette::DARK;
        assert_eq!(palette.smartness_color(0.0), palette.smartness[0]);
        assert_eq!(palette.smartness_color(100.0), palette.smartness[3]);
        assert_eq!(palette.smartness_color(-5.0), palette.smartness[0]);
        assert_eq!(palette.smartness_color(150.0), palette.smartness[3]);
        let middle = palette.smartness_color(100.0 / 3.0);
        assert_eq!(middle, palette.smartness[1]);
    }

    #[test]
    fn all_digits_lay_out_at_the_same_width() {
        // egui's default font has tabular digits too, so check first that Inter is the one
        // laying them out: upstream Inter's digits have nine different widths.
        assert_eq!(fonts().families[&FontFamily::Proportional][0], "inter");
        let context = egui::Context::default();
        install(&context);
        // Fonts load on the first frame; there is no renderer here to take the textures.
        let mut output = context.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        let widths: Vec<f32> = context.fonts_mut(|fonts| {
            ('0'..='9')
                .map(|digit| {
                    fonts
                        .layout_no_wrap(
                            digit.to_string(),
                            FontId::proportional(14.0),
                            Color32::WHITE,
                        )
                        .size()
                        .x
                })
                .collect()
        });
        assert!(widths.iter().all(|width| *width == widths[0]), "{widths:?}");
    }

    #[test]
    fn both_themes_follow_the_system_by_default() {
        let context = egui::Context::default();
        install(&context);
        assert_eq!(
            context.options(|options| options.theme_preference),
            ThemePreference::System
        );
        context.set_theme(Theme::Light);
        assert_eq!(palette(&context), &Palette::LIGHT);
        context.set_theme(Theme::Dark);
        assert_eq!(palette(&context), &Palette::DARK);
    }
}
