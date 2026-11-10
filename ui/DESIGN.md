# DESIGN.md: the graphical UI

The design choices for the `genetic_algo_ui` crate, and the references behind them. Rules for working in the repository are in [AGENTS.md](../AGENTS.md); how the logo is made is in [assets/logo/AGENTS.md](../assets/logo/AGENTS.md).

Status marks: **done** is in the code, **planned** is agreed but not built yet.

## Product character
An experiment bench, not a game and not a decorative dashboard. The map is the subject; the rules stay visible while a run plays, because tuning them live and watching the effect is the point. Native, restrained, readable. Borrowed from ZapFast's product rules: keep keyboard operation, zoom, light and dark themes, and narrow windows usable.

## Layout: "Lab bench" (done)
Chosen on 2026-11-07 from three mocked directions (A "Lab bench", B "Focus canvas", C "Dashboard tiles"). A keeps the three columns of the earlier UI and refines them. One module per region:
- **Toolbar** (`toolbar.rs`) across the top: logo, play/pause (the one accent-filled button) and step as icon buttons, speed, tick / generation / population / mean-smartness readouts, and a theme toggle cycling system, light and dark. Under it, a status strip says why a run stopped or why an edit was refused, only when there is something to say.
- **Left, rules** (`rules_panel.rs`): a World card for the next run (size, start smart, seed, new run, replay), the presets, then one card per group of rules. A rule that is either on or off (leaders, brains, the goal) has a switch in its card's header, and its settings show only while it is on. Other on/off settings are switches too.
- **Centre, map** (`map_view.rs`): shapes carry identity, brightness carries smartness, a ring marks the leader. The map stays dark in both themes. Hovering shows a quick tooltip; clicking an individual selects it, outlined in white.
- **Inspector** (`inspector.rs`), taken from B: a card over the map's top-right corner for the selected individual. It follows the individual by id as it moves and says when it has died. A new run drops the selection, because ids restart with each run.
- **Right, stats and charts** (`stats_panel.rs`, `charts.rs`): tiles for population and mean smartness (and speed when it can evolve) with sparklines of the last 600 ticks, taken from C; a card with births, meals, fights and deaths split by cause in one bar; the leadership when leaders are on; then one card per chart. The mean-smartness line takes the logo's gradient by value. Charts are at least 140 px tall and the panel scrolls when they do not fit.
- **Toasts** (`egui-notify`), taken from B: when a run finishes, a toast says why for six seconds.

Differences from the mockup, on purpose:
- **No reset button in the toolbar.** "New random run" and "Replay seed" stay in the World card, next to the settings they apply; two buttons with the same name would also confuse screen readers.
- **Cards do not collapse.** A switch already hides a rule's settings while it is off, and the panel scrolls; collapsing the always-on groups would add state for little gain.
- **The toast cannot be closed by hand.** egui-notify draws its close button with a glyph the bundled fonts lack, so it is turned off; the toast leaves on its own and the status strip keeps the message.

Accessibility: icon buttons and switches name themselves (`widgets.rs`), and eframe's `accesskit` feature is on, so screen readers and the interaction tests find controls by those names. It costs about 5.6 MB of binary.

Why not B: it hides the rules while a run plays. Why not C: the most code (`egui_tiles`, saved layouts) for the smallest map. C's resizable panes stay an option if fixed panels prove limiting.

## Theme (done)
- **Colour roles,** from fastframe-theme: `window`, `panel`, `surface`, `surface_hover`, `surface_active`, `outline`, `text`, `secondary`, `dim`, `accent`, `accent_hover`, `on_accent`, `danger`, `warning`, plus chart series colours. One token struct feeds `egui::Visuals`; outside the map, nothing else hard-codes a colour. The map's cells keep the core's palette (shared with the terminal and GIF output) and its marks are fixed colours, because the map is dark in both themes.
- **Mapping onto egui,** after ZapFast's `theme.rs`: widget states are `surface` / `surface_hover` / `surface_active` with no border; non-interactive frames get a 1 px `outline`; selection is `accent` at 35%; corner radius 8 for windows, menus and cards, 6 for widgets. Spacing is tighter than ZapFast's (`button_padding` 10x4 instead of 12x6, `interact_size` height 24 instead of 28), because the rules panel is dense.
- **Light and dark,** following the system (`ThemePreference::System`), with a manual toggle. A palette can be derived from background, text and accent with fastframe's recipe: panel = mix(bg, fg, 3%), surface 8%, hover 12%, active 18%, outline 20%, secondary 70%, dim 50%.
- **Identity colours** come from the logo: navy `#00163D`, then teal `#00D1E0`, green `#83E088`, yellow `#FFD21F`, orange `#FE9A0D`. The dark theme derives from a navy background, the light one from a near-white; both use the logo's teal as the accent and its gradient for smartness, darkened in the light theme where it would wash out.
- **Contrast is tested,** in `theme.rs`: text at least 7:1 and secondary text 4.5:1 on every surface, accents and the smartness colours at least 3:1, in both themes. The light theme's secondary text is darker than the recipe's 70%, which fell short on pressed widgets.

## Typography (done)
- **Inter** 4.1 (OFL-1.1, licence in `assets/fonts/`) with tabular figures frozen into the font, so counters that change every frame do not jitter. egui cannot select OpenType features, hence the frozen copy, rebuilt from the upstream release (`assets/fonts/README.md`). Regular (400) for text, semibold (600) for headings and stat values, both from the one variable file via `VariationCoords`; egui's own fonts stay behind it as fallbacks.
- **Text rendering** matched to the desktop, after fastframe-text: on Linux, egui's dark-mode coverage curve draws text 11-18% heavier than GTK, and linear coverage comes within 2%; macOS renders unhinted; Windows keeps egui's defaults. No runtime detection of the desktop's settings (fastframe reads them over D-Bus), only these per-platform defaults. The adjustment is part of building the visuals, so replacing the visuals keeps it.

## Icons
- **App icon (done):** generated from `assets/logo/`. The window icon is set in `identity.rs`, except on macOS, where the bundle's icon must win in the Dock.
- **UI icons (done):** `egui_material_icons`, an icon font registered behind Inter, so no SVG loader is needed. Lucide SVGs through a copy of fastframe's `icons!` macro remain the alternative for an outline style; that needs `egui_extras` with `svg`.
- **In-app logo (done):** the toolbar draws `assets/icons/icon-32.png`, rendered from the 3x3 variant that stays legible at that size.

## Identity and installation
- **Done:** one ID, `io.github.riccardogabellone.GeneticAlgo`, for the Wayland app id, the `.desktop` file name, its `Icon` and `StartupWMClass` (a test checks they agree). Release builds on Windows open no console; `build.rs` embeds the `.ico` in the executable.
- **Done:** `cargo-packager` builds `.deb` and `.AppImage` on Linux (ubuntu-22.04, for an older glibc floor), NSIS and MSI installers plus a portable zip on Windows, and a `.dmg` on macOS. A `v*` tag publishes them with `checksums.txt` and build provenance; pull requests that touch packaging build them without publishing. The packages ship the desktop entry, AppStream metadata and icons named after the ID, because cargo-packager's generated ones are named after the binary. CI builds on all three systems and requires the Windows icon to be embedded. Details: [packaging/README.md](../packaging/README.md).
- **Fallback** if cargo-packager stalls (no release since late 2025): cargo-deb, a short macOS bundle script after ZapFast's, cargo-wix.
- **Distribution:** GitHub releases only; publishing to Flathub, winget or Homebrew is not planned for now (decided 2026-11-07).
- **Later:** signing (Apple Developer ID, Azure Artifact Signing). Until then, macOS Gatekeeper and Windows SmartScreen warn on first launch.

## Rendering
OpenGL (eframe's `glow`) on Linux and macOS; wgpu on Windows, on DirectX 12 or Vulkan. `ui/Cargo.toml` compiles in one renderer per system and `ui/src/renderer.rs` configures it.

- **Why not OpenGL on Windows:** its driver took more memory with every frame drawn, so the process grew for as long as a run played: about 20 KB per frame on an NVIDIA GPU, which is over 1 MB per second and 2 GB within half an hour. None of it was the app's data: the heap stayed near 5 MB throughout. wgpu stays flat.
- **The cost:** wgpu starts higher, at about 130 to 210 MB resident depending on the GPU, against under 100 MB for OpenGL. It is asked to save memory (`MemoryHints::MemoryUsage`), which halves what it reserves.
- **Why not wgpu everywhere:** OpenGL measured flat on Linux, and wgpu could not create a surface on WSL, which has no Vulkan loader.
- **Checking it:** `cargo run --release -p genetic_algo_ui --example memory` plays runs back to back in the real window and fails if the heap or the process keeps growing. Run it on the system in question: the leak never showed on Linux.

## Dependencies and the egui version
The UI is on egui / eframe 0.37. Before adding an egui crate, check its egui requirement on crates.io; a crate built for another egui version brings a second, incompatible copy of egui.

| Crate | egui | Use |
|---|---|---|
| egui_plot 0.38 | 0.37 | charts (in use; has `Line::gradient_color` and `Line::fill`) |
| egui_material_icons 0.9 | 0.37 | UI icons (in use) |
| egui-notify 0.24 | 0.37 | toasts (in use) |
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
