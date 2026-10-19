# Searchable plugin menus

The October 17 usability update replaces the plugin menu's unlimited columns with a compact, scrollable picker.
The upstream plugin database, scanning, visibility/format preferences and add/replace commands remain authoritative.
This is a presentation/input change, not another plugin registry or hosting backend.

## Using it

- Open the plugin menu with F8, the toolbar plugin button, the channel rack's `+`, an empty mixer slot, or an
  instrument/effect submenu such as Insert, Replace or ADD > Effect. F8 follows the existing shortcut preferences.
- Start typing immediately. Search is case-insensitive; all space-separated words must match across the plugin's
  name, vendor, category or format. For example, `lemon eq` finds Lemon Parametric EQ; `clap` filters by format.
- Scroll with the mouse wheel or click the scrollbar track to page. Up/Down, Page Up/Down, Home and End move the
  keyboard selection; Enter adds the selected result. Click a result to choose it directly.
- Category, Vendor and A–Z change grouping without closing the picker; favorites remain first. The grouping choice
  uses upstream's existing persisted preference. Each result identifies instrument/effect, group, vendor and format.
- Ctrl+A selects the query, typing replaces it, Backspace edits it, and the `×` clears it. Esc first clears a
  nonempty query; Esc again closes the menu. The picker shortcut also closes it. Manage opens the existing plugin
  manager, including its scan controls.

Effect slots still list only effects, generator menus only instruments, and an all-plugin menu distinguishes both.
Filtered picks retain the original mixer slot or rack insertion position. Typing while searching must not play
notes, start transport, save or edit a background window; releases of previously held typing notes still get through.

## Sizing and rendering

The panel is capped at 450 menu-space pixels wide and 650 high, then constrained to the available window. Long
lists scroll instead of adding off-screen columns. Popup scaling is honored unless it would leave less than a
280×180 menu-space viewport; in that case its scale is reduced to keep search, results and controls usable.
The regular app overlay path still clips docked editors beneath the panel, and lifted plugin menus use the same
renderer. No compositor rules, native-window backend or plugin DSP changed.

## Regression coverage

- A 1,000-plugin synthetic library: multiword/case-insensitive search, vendor/category metadata, favorites,
  nested vendor entries, empty results and effect/generator filtering.
- Preserved explicit mixer slots and rack insertion positions; filtered mouse selection and keyboard effect
  insertion; the inherited ADD > Effect test still checks first-empty-slot selection and a full mixer track.
- Search consumes typing/shortcuts, releases pre-existing held notes, handles Unicode deletion, clears/closes,
  navigates and scrolls without changing the project.
- Click bounds in Classic and Lemon at 640×480, 800×600 and 2544×1384, including 2x main scaling, independent 150%
  popup scaling and the 400% popup-setting fallback.
- Optional `OS_SHOT_DIR` snapshots for browsing, filtered and empty-result states. These were generated and
  visually inspected for this update, including Lemon's actual color/style variant.

Run `cargo test -p os-app plugin_menu --locked --offline` for the focused tests, and the inherited
`add_effect_submenu_fills_the_selected_track` test for menu-bar integration. Full Linux validation and installed
artifact details are recorded in the project's Obsidian plugin-search note. Private FLP corpus fixtures, arbitrary
third-party plugins, Windows/macOS and hands-on Hyprland focus/monitor acceptance remain separate checks.

The earlier pattern `+` behavior and uncertain maximized-mixer clipping report were not changed by this update.
