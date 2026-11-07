# DESIGN.md: the graphical UI

The design choices for the `genetic_algo_ui` crate, and the references behind them. Rules for working in the repository are in [AGENTS.md](../AGENTS.md); how the logo is made is in [assets/logo/AGENTS.md](../assets/logo/AGENTS.md).

Status marks: **done** is in the code, **planned** is agreed but not built yet.

## Product character
An experiment bench, not a game and not a decorative dashboard. The map is the subject; the rules stay visible while a run plays, because tuning them live and watching the effect is the point. Native, restrained, readable. Borrowed from ZapFast's product rules: keep keyboard operation, zoom, light and dark themes, and narrow windows usable.

## Layout: "Lab bench" (planned)
Chosen on 2026-11-07 from three mocked directions (A "Lab bench", B "Focus canvas", C "Dashboard tiles"). A keeps the three columns of the current UI and refines them:
- **Toolbar** across the top: logo, play/pause, step, reset as icon buttons, speed, tick / generation / population readouts, seed, theme toggle.
- **Left, rules:** collapsible section cards (title, then a card of rows: label, dim one-line description, control on the right). On/off rules use switches, not checkboxes. A collapsed card shows a summary of its values.
- **Centre, map:** as today (shapes carry identity, brightness carries smartness, ring marks the leader). The map stays dark in both app themes. Clicking an individual selects it.
- **Right, stats and charts:** stat tiles with sparklines (taken from C), then stacked charts. The smartness line uses the logo's gradient.
- **Inspector card** for the selected individual, replacing today's hover-only tooltip. A hover card stays for quick looks (taken from B).
- **Toasts** for events such as "target reached" (taken from B).

Why not B: it hides the rules while a run plays. Why not C: the most code (`egui_tiles`, saved layouts) for the smallest map. C's resizable panes stay an option if fixed panels prove limiting.

## Theme (planned)
- **Colour roles,** from fastframe-theme: `window`, `panel`, `surface`, `surface_hover`, `surface_active`, `outline`, `text`, `secondary`, `dim`, `accent`, `accent_hover`, `on_accent`, `danger`, `warning`, plus chart series colours. One token struct feeds `egui::Visuals`; nothing else hard-codes a colour.
- **Mapping onto egui,** after ZapFast's `theme.rs`: widget states are `surface` / `surface_hover` / `surface_active` with no border; non-interactive frames get a 1 px `outline`; selection is `accent` at 35%; corner radius 8 for windows and menus, 6 for widgets; `button_padding` 12x6, `interact_size` height 28.
- **Light and dark,** following the system (`ThemePreference::System`), with a manual toggle. A palette can be derived from background, text and accent with fastframe's recipe: panel = mix(bg, fg, 3%), surface 8%, hover 12%, active 18%, outline 20%, secondary 70%, dim 50%.
- **Identity colours** come from the logo: navy `#00163D`, then teal `#00D1E0`, green `#83E088`, yellow `#FFD21F`, orange `#FE9A0D`.

## Typography (planned)
- **Inter** (OFL-1.1, ship its licence) with tabular figures frozen into the font, so counters that change every frame do not jitter. egui cannot select OpenType features, hence the frozen copy (`pyftfeatfreeze -f tnum`). Weights from the one variable file via `VariationCoords`.
- **Text rendering** matched to the desktop, copied from fastframe-text: egui's dark-mode coverage curve draws text 11-18% heavier than GTK; linear coverage comes within 2%. Re-apply after every `set_visuals`, which resets these fields.

## Icons
- **App icon (done):** generated from `assets/logo/`. The window icon is set in `identity.rs`, except on macOS, where the bundle's icon must win in the Dock.
- **UI icons (planned):** `egui_material_icons` (an icon font, no SVG loader needed). Lucide SVGs through a copy of fastframe's `icons!` macro are the alternative if we want its outline style; that needs `egui_extras` with `svg`.

## Identity and installation
- **Done:** one ID, `io.github.riccardogabellone.GeneticAlgo`, for the Wayland app id, the `.desktop` file name, its `Icon` and `StartupWMClass` (a test checks they agree). Release builds on Windows open no console; `build.rs` embeds the `.ico` in the executable.
- **Planned:** `cargo-packager` for deb, AppImage, msi/nsis, app and dmg from one config, and a GitHub Actions release on `v*` tags building on Linux (ubuntu-22.04, for an older glibc floor), Windows and macOS, with `checksums.txt` and build provenance. Fallback if cargo-packager stalls (no release since late 2025): cargo-deb, a short macOS bundle script after ZapFast's, cargo-wix. Later: signing (Apple Developer ID, Azure Artifact Signing), Flathub, winget, Homebrew. Until then, macOS Gatekeeper and Windows SmartScreen warn on first launch.

## Dependencies and the egui version
The UI is on egui / eframe 0.37. Before adding an egui crate, check its egui requirement on crates.io; a crate built for another egui version brings a second, incompatible copy of egui.

| Crate | egui | Use |
|---|---|---|
| egui_plot 0.38 | 0.37 | charts (in use; has `Line::gradient_color` and `Line::fill`) |
| egui_material_icons 0.9 | 0.37 | UI icons (planned) |
| egui-notify 0.24 | 0.37 | toasts (planned) |
| egui_tiles 0.18 | 0.37 | resizable panes, only if layout A proves limiting |
| egui_dock 0.21 | 0.36 | not usable yet |
| catppuccin-egui 5.7 | 0.33 | stale: copy hex values instead |

**fastframe is a reference, not a dependency.** Its crates pin egui 0.36.2, so they cannot join an egui 0.37 build. Its code is MIT: copy the small pure modules (text-rendering mapping, `icons!` macro, palette parsing) with attribution in the file header. Its egui-free crates (`fastframe-log`, `-dirs`, `-instance`, `-update`) could be used directly if ever needed.

## References
- fastframe, egui desktop foundation: https://fastframe.dev/, https://github.com/crmne/fastframe (guides: what-is-fastframe, moving-apps, per-crate pages)
- ZapFast, the most complete fastframe app: https://github.com/crmne/zapfast (`PRODUCT.md`, `src/theme.rs`, `src/ui/settings.rs`, `src/ui/widgets.rs`, `packaging/`, `build.rs`)
- native-packages, fastframe apps' packaging tool: https://github.com/crmne/native-packages
- Rerun's design system, for token values and widget patterns: `re_ui` on crates.io (`design_tokens.rs`, `list_item`, `section_collapsing_header`, `command_palette`)
- egui_tiles: https://github.com/rerun-io/egui_tiles
- cargo-packager: https://github.com/crabnebula-dev/cargo-packager
- freedesktop desktop entries and icon theme: https://specifications.freedesktop.org/desktop-entry-spec/latest/, https://specifications.freedesktop.org/icon-theme-spec/latest/
- Mockups of the three directions (private page, owner only): https://claude.ai/artifact/NRxshx4w25qN7FG9wnEEVk
