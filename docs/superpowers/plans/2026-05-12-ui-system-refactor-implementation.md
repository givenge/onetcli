# UI System Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a unified dense desktop UI system for OnetCli, then land it through Story, Command Home, and the main workbench shells without changing connection, sync, terminal, SFTP, database, or AI business behavior.

**Architecture:** First stabilize shared theme tokens and reusable chrome helpers in `gpui-component` / `one-ui`. Then split `HomePage` rendering into focused `main/src/home/*` modules and use the new helpers for the Command Home sample. Finally apply shared workbench chrome to database, Redis, MongoDB, terminal, SFTP, and settings surfaces as a visual alignment pass.

**Tech Stack:** Rust 2024, GPUI, `gpui-component`, `one-ui`, `one_core`, `gpui-component-story`, Cargo checks and GPUI unit tests.

---

## Scope And Guardrails

- Keep storage models, repository APIs, connection opening, sync engine, auth, license, terminal execution, SFTP transfer, SQL execution, Redis commands, MongoDB commands, and ChatDB request semantics unchanged.
- Use `Default Light` and `macOS Classic Dark` as the tuned themes. Other themes must keep parsing and rendering with existing fallback behavior.
- Use theme tokens and helper components instead of page-local color constants where practical.
- Keep all new Rust source files under 300 lines and new functions under 50 lines.
- Run `superpowers:test-driven-development` before implementation because this changes shared UI behavior and cross-module rendering structure.
- Run `superpowers:verification-before-completion` before any completion, commit, push, or PR claim.

## File Map

### Shared Design System

- Modify `crates/ui/src/theme/default-theme.json`: tune the two selected themes and add missing values for existing schema keys.
- Modify `crates/ui/src/button/button.rs`: make disabled and ghost states token-driven and test bounded alpha values.
- Create `crates/ui/src/chrome.rs`: shared desktop chrome primitives for rail, toolbar, panel, section header, and status pill.
- Modify `crates/ui/src/lib.rs`: export `chrome`.
- Create `crates/ui/src/chrome_tests.rs` only if the chrome helpers need test-only assertions that are cleaner outside `chrome.rs`; prefer inline `#[cfg(test)]` tests in `chrome.rs`.
- Create `crates/story/src/stories/design_system_story.rs`: visual matrix for buttons, inputs, tabs, lists, tables, popovers, dialogs, rails, toolbars, and status pills.
- Modify `crates/story/src/stories/mod.rs`: export `DesignSystemStory`.
- Modify `crates/story/src/main.rs`: add `DesignSystemStory` near `ThemeColorsStory`.

### App-Specific Workbench Helpers

- Create `crates/one_ui/src/workbench.rs`: reusable three-column workbench shell helpers.
- Modify `crates/one_ui/src/lib.rs`: export `workbench` helpers and keep `edit_table::init(cx)`.

### Command Home

- Modify `main/src/home/mod.rs`: register new Home rendering modules.
- Create `main/src/home/connection_display.rs`: pure display helpers for connection subtitles, type icons, team badges, and grouping.
- Create `main/src/home/home_toolbar.rs`: toolbar, search, sync, key, conflict, refresh, and workspace-filter trigger.
- Create `main/src/home/home_navigation.rs`: global rail and connection-type navigation.
- Create `main/src/home/home_content.rs`: workspace groups, empty states, filtered-empty states, and scroll body.
- Create `main/src/home/connection_card.rs`: connection card rendering and hover actions.
- Modify `main/src/home/home_workspace_filter.rs`: align popover list styling with shared dense chrome.
- Modify `main/src/home_tab.rs`: keep state and behavior, expose the minimal `pub(crate)` methods needed by the new rendering modules, and delegate render methods to those modules.
- Modify `main/locales/main.yml`: add user-facing strings for Command Home empty and filtered states.

### Workbench Alignment

- Modify `crates/db_view/src/database_tab.rs`: use `one_ui::workbench` for resource panel, main panel, right AI/context panel, and toolbar heights.
- Modify `crates/redis_view/src/redis_tab.rs` and `crates/redis_view/src/sidebar.rs`: align Redis tree and detail panel chrome.
- Modify `crates/mongodb_view/src/mongo_tab.rs` and `crates/mongodb_view/src/sidebar.rs`: align Mongo tree and collection panel chrome.
- Modify `crates/terminal_view/src/view.rs` and `crates/terminal_view/src/sidebar/mod.rs`: align terminal main area, side panels, and AI/context affordance.
- Modify `crates/sftp_view/src/file_list_panel.rs`: align path/search toolbar and file table chrome.
- Modify `main/src/setting_tab.rs` and `main/src/new_connection/*`: apply the same toolbar and panel spacing where it does not change form behavior.

---

## Task 1: Shared Chrome Primitives And Button State Tests

**Files:**
- Modify: `crates/ui/src/button/button.rs`
- Create: `crates/ui/src/chrome.rs`
- Modify: `crates/ui/src/lib.rs`

- [ ] **Step 1: Add failing button interaction tests**

Add this inside the existing `#[cfg(test)] mod tests` in `crates/ui/src/button/button.rs`:

```rust
#[gpui::test]
fn test_button_disabled_styles_keep_alpha_bounded(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        crate::init(cx);
        let secondary = ButtonVariant::Secondary.disabled(false, cx);
        let primary = ButtonVariant::Primary.disabled(false, cx);

        assert!(secondary.bg.a <= 1.0);
        assert!(secondary.border.a <= 1.0);
        assert!(primary.bg.a <= 1.0);
        assert!(primary.border.a <= 1.0);
    });
}

#[gpui::test]
fn test_text_and_ghost_buttons_use_theme_transparent_background(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        crate::init(cx);
        let text = ButtonVariant::Text.normal(false, cx);
        let ghost = ButtonVariant::Ghost.normal(false, cx);

        assert_eq!(text.bg, cx.theme().transparent);
        assert_eq!(ghost.bg, cx.theme().transparent);
    });
}
```

- [ ] **Step 2: Run the focused test and confirm the red state**

Run:

```bash
cargo test -p gpui-component button::button::tests::test_button_disabled_styles_keep_alpha_bounded -- --nocapture
```

Expected: FAIL before the implementation because the secondary disabled style currently uses `cx.theme().secondary.opacity(1.5)`.

- [ ] **Step 3: Fix button disabled style**

In `crates/ui/src/button/button.rs`, change the `Self::Secondary` arm in `fn disabled` to a bounded token style:

```rust
Self::Secondary => cx.theme().secondary.opacity(0.55),
```

- [ ] **Step 4: Add shared chrome helpers**

Create `crates/ui/src/chrome.rs`:

```rust
use gpui::{App, Div, ParentElement as _, Styled as _, div, px};

use crate::{ActiveTheme, h_flex, v_flex};

pub const APP_RAIL_WIDTH: f32 = 52.0;
pub const APP_TOOLBAR_HEIGHT: f32 = 44.0;
pub const PANEL_HEADER_HEIGHT: f32 = 36.0;
pub const DENSE_ROW_HEIGHT: f32 = 30.0;

pub fn app_rail(cx: &App) -> Div {
    v_flex()
        .w(px(APP_RAIL_WIDTH))
        .h_full()
        .flex_shrink_0()
        .items_center()
        .gap_1()
        .py_2()
        .bg(cx.theme().sidebar)
        .border_r_1()
        .border_color(cx.theme().sidebar_border)
}

pub fn app_toolbar(cx: &App) -> Div {
    h_flex()
        .h(px(APP_TOOLBAR_HEIGHT))
        .min_h(px(APP_TOOLBAR_HEIGHT))
        .w_full()
        .items_center()
        .gap_2()
        .px_3()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
}

pub fn panel_surface(cx: &App) -> Div {
    v_flex()
        .size_full()
        .min_w_0()
        .min_h_0()
        .bg(cx.theme().background)
        .text_color(cx.theme().foreground)
}

pub fn panel_header(cx: &App) -> Div {
    h_flex()
        .h(px(PANEL_HEADER_HEIGHT))
        .min_h(px(PANEL_HEADER_HEIGHT))
        .w_full()
        .items_center()
        .gap_2()
        .px_3()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().muted)
}

pub fn content_canvas(cx: &App) -> Div {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .overflow_hidden()
        .bg(cx.theme().muted)
}

pub fn status_pill(cx: &App) -> Div {
    h_flex()
        .h_5()
        .items_center()
        .gap_1()
        .px_2()
        .rounded(px(5.0))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary)
        .text_xs()
        .text_color(cx.theme().secondary_foreground)
}
```

- [ ] **Step 5: Export chrome helpers**

Modify `crates/ui/src/lib.rs`:

```rust
pub mod chrome;
```

- [ ] **Step 6: Run focused and crate checks**

Run:

```bash
cargo test -p gpui-component button::button::tests::test_button_disabled_styles_keep_alpha_bounded -- --nocapture
cargo test -p gpui-component button::button::tests::test_text_and_ghost_buttons_use_theme_transparent_background -- --nocapture
cargo check -p gpui-component
```

Expected: all commands exit 0.

- [ ] **Step 7: Commit Task 1**

```bash
git add crates/ui/src/button/button.rs crates/ui/src/chrome.rs crates/ui/src/lib.rs
git commit -m "refactor(ui): add shared chrome primitives"
```

---

## Task 2: Theme Token Coverage And Story Matrix

**Files:**
- Modify: `crates/ui/src/theme/schema.rs`
- Modify: `crates/ui/src/theme/default-theme.json`
- Create: `crates/story/src/stories/design_system_story.rs`
- Modify: `crates/story/src/stories/mod.rs`
- Modify: `crates/story/src/main.rs`

- [ ] **Step 1: Add a theme coverage test**

Add this test module to `crates/ui/src/theme/schema.rs`:

```rust
#[cfg(test)]
mod tests {
    use serde_json::Value;

    const REQUIRED_THEME_KEYS: &[&str] = &[
        "background",
        "foreground",
        "border",
        "muted.background",
        "muted.foreground",
        "secondary.background",
        "secondary.foreground",
        "secondary.hover.background",
        "secondary.active.background",
        "primary.background",
        "primary.foreground",
        "primary.hover.background",
        "primary.active.background",
        "danger.background",
        "danger.foreground",
        "danger.hover.background",
        "danger.active.background",
        "warning.background",
        "warning.foreground",
        "warning.hover.background",
        "warning.active.background",
        "success.background",
        "success.foreground",
        "success.hover.background",
        "success.active.background",
        "sidebar.background",
        "sidebar.foreground",
        "sidebar.border",
        "sidebar.accent.background",
        "sidebar.accent.foreground",
        "tab.background",
        "tab.active.background",
        "tab.active.foreground",
        "tab_bar.background",
        "table.background",
        "table.head.background",
        "table.head.foreground",
        "table.hover.background",
        "table.active.background",
        "table.active.border",
        "table.row.border",
        "popover.background",
        "popover.foreground",
        "overlay.background",
        "ring",
        "input.border",
    ];

    #[test]
    fn default_light_and_classic_dark_define_required_ui_tokens() {
        let value: Value = serde_json::from_str(include_str!("default-theme.json")).unwrap();
        let themes = value["themes"].as_array().unwrap();

        for name in ["Default Light", "macOS Classic Dark"] {
            let theme = themes
                .iter()
                .find(|theme| theme["name"] == name)
                .unwrap_or_else(|| panic!("missing theme {name}"));
            let colors = theme["colors"].as_object().unwrap();
            for key in REQUIRED_THEME_KEYS {
                assert!(colors.contains_key(*key), "{name} missing color key {key}");
            }
        }
    }
}
```

- [ ] **Step 2: Run the coverage test and confirm the red state**

Run:

```bash
cargo test -p gpui-component theme::schema::tests::default_light_and_classic_dark_define_required_ui_tokens -- --nocapture
```

Expected: FAIL before the JSON update because several required keys are missing in one or both target themes.

- [ ] **Step 3: Update theme token values**

In `crates/ui/src/theme/default-theme.json`, update the `colors` object for `Default Light` with these values:

```json
"background": "#ffffff",
"foreground": "#1f2937",
"border": "#d9dee7",
"ring": "#2563eb",
"input.border": "#cfd6e2",
"muted.background": "#f4f6f8",
"muted.foreground": "#667085",
"secondary.background": "#eef2f7",
"secondary.foreground": "#344054",
"secondary.hover.background": "#e3e8f0",
"secondary.active.background": "#d7deea",
"primary.background": "#2563eb",
"primary.foreground": "#ffffff",
"primary.hover.background": "#1d4ed8",
"primary.active.background": "#1e40af",
"danger.background": "#dc2626",
"danger.foreground": "#ffffff",
"danger.hover.background": "#b91c1c",
"danger.active.background": "#991b1b",
"warning.background": "#d97706",
"warning.foreground": "#ffffff",
"warning.hover.background": "#b45309",
"warning.active.background": "#92400e",
"success.background": "#16803a",
"success.foreground": "#ffffff",
"success.hover.background": "#116c31",
"success.active.background": "#0f5f2c",
"sidebar.background": "#f7f8fa",
"sidebar.foreground": "#344054",
"sidebar.border": "#d9dee7",
"sidebar.accent.background": "#e8eef8",
"sidebar.accent.foreground": "#1f2937",
"sidebar.primary.background": "#2563eb",
"sidebar.primary.foreground": "#ffffff",
"list.background": "#ffffff",
"list.even.background": "#f8fafc",
"list.hover.background": "#edf2f7",
"list.active.background": "#dbeafe",
"list.active.border": "#2563eb",
"list.head.background": "#f1f4f8",
"tab.background": "#eef2f7",
"tab.active.background": "#ffffff",
"tab.active.foreground": "#1f2937",
"tab_bar.background": "#f7f8fa",
"tab_bar.segmented.background": "#e6ebf2",
"table.background": "#ffffff",
"table.head.background": "#f1f4f8",
"table.head.foreground": "#475467",
"table.hover.background": "#edf2f7",
"table.active.background": "#dbeafe",
"table.active.border": "#2563eb",
"table.row.border": "#e4e7ec",
"popover.background": "#ffffff",
"popover.foreground": "#1f2937",
"overlay.background": "#11182780",
"title_bar.background": "#ffffff",
"title_bar.border": "#d9dee7",
"selection.background": "#bfdbfe"
```

Update `macOS Classic Dark` with these values:

```json
"background": "#171717",
"foreground": "#e6e8eb",
"border": "#373a40",
"ring": "#5b8def",
"input.border": "#3f434b",
"muted.background": "#202124",
"muted.foreground": "#a1a7b3",
"secondary.background": "#2a2d33",
"secondary.foreground": "#e6e8eb",
"secondary.hover.background": "#343842",
"secondary.active.background": "#3d4350",
"primary.background": "#4f83ff",
"primary.foreground": "#ffffff",
"primary.hover.background": "#6b96ff",
"primary.active.background": "#3d6fe0",
"danger.background": "#ef4444",
"danger.foreground": "#ffffff",
"danger.hover.background": "#f87171",
"danger.active.background": "#dc2626",
"warning.background": "#d99a22",
"warning.foreground": "#171717",
"warning.hover.background": "#e7ad3a",
"warning.active.background": "#b7791f",
"success.background": "#3ca45c",
"success.foreground": "#ffffff",
"success.hover.background": "#4fb86e",
"success.active.background": "#2f8c4d",
"sidebar.background": "#1e1f22",
"sidebar.foreground": "#d7dce3",
"sidebar.border": "#30343a",
"sidebar.accent.background": "#2b313b",
"sidebar.accent.foreground": "#f5f7fa",
"sidebar.primary.background": "#4f83ff",
"sidebar.primary.foreground": "#ffffff",
"list.background": "#1b1c1f",
"list.even.background": "#202124",
"list.hover.background": "#272b33",
"list.active.background": "#233859",
"list.active.border": "#5b8def",
"list.head.background": "#20242a",
"tab.background": "#202124",
"tab.active.background": "#2a2d33",
"tab.active.foreground": "#f5f7fa",
"tab_bar.background": "#171717",
"tab_bar.segmented.background": "#202124",
"table.background": "#18191c",
"table.head.background": "#20242a",
"table.head.foreground": "#b7beca",
"table.hover.background": "#252a32",
"table.active.background": "#233859",
"table.active.border": "#5b8def",
"table.row.border": "#2d3138",
"popover.background": "#26282d",
"popover.foreground": "#f5f7fa",
"overlay.background": "#00000099",
"title_bar.background": "#171717",
"title_bar.border": "#30343a",
"selection.background": "#28476f"
```

- [ ] **Step 4: Add `DesignSystemStory`**

Create `crates/story/src/stories/design_system_story.rs`:

```rust
use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Window, div, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    chrome,
    h_flex,
    input::{Input, InputState},
    tab::{Tab, TabBar},
    v_flex,
};

use crate::section;

pub struct DesignSystemStory {
    search: Entity<InputState>,
}

impl super::Story for DesignSystemStory {
    fn title() -> &'static str {
        "Design System"
    }

    fn description() -> &'static str {
        "Unified desktop chrome, states, and density."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        cx.new(|cx| Self {
            search: cx.new(|cx| InputState::new(window, cx).placeholder("Search connections")),
        })
    }
}

impl Render for DesignSystemStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("design-system-story")
            .size_full()
            .gap_5()
            .child(
                section("Chrome")
                    .child(
                        h_flex()
                            .h(px(220.0))
                            .w_full()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius_lg)
                            .overflow_hidden()
                            .child(
                                chrome::app_rail(cx)
                                    .child(Icon::new(IconName::Home).small())
                                    .child(Icon::new(IconName::Database).small())
                                    .child(Icon::new(IconName::Settings).small()),
                            )
                            .child(
                                chrome::panel_surface(cx)
                                    .child(
                                        chrome::app_toolbar(cx)
                                            .child(Button::new("new").primary().label("New"))
                                            .child(Input::new(&self.search).w(px(240.0)))
                                            .child(div().flex_1())
                                            .child(chrome::status_pill(cx).child("Synced")),
                                    )
                                    .child(
                                        chrome::content_canvas(cx).p_3().child(
                                            chrome::panel_surface(cx)
                                                .border_1()
                                                .border_color(cx.theme().border)
                                                .rounded(cx.theme().radius)
                                                .child(chrome::panel_header(cx).child("Panel"))
                                                .child(div().p_3().child("Dense content area")),
                                        ),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Controls")
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_wrap()
                            .child(Button::new("primary").primary().label("Primary"))
                            .child(Button::new("secondary").label("Secondary"))
                            .child(Button::new("ghost").ghost().label("Ghost"))
                            .child(Button::new("danger").danger().label("Danger"))
                            .child(Button::new("disabled").label("Disabled").disabled(true)),
                    )
                    .child(
                        TabBar::new("tabs")
                            .selected_index(0)
                            .child(Tab::new().label("Data"))
                            .child(Tab::new().label("AI")),
                    ),
            )
    }
}
```

- [ ] **Step 5: Register the story**

Modify `crates/story/src/stories/mod.rs`:

```rust
mod design_system_story;
pub use design_system_story::*;
```

Modify the `Components` story list in `crates/story/src/main.rs` by adding:

```rust
StoryContainer::panel::<DesignSystemStory>(window, cx),
```

Place it immediately after `ThemeColorsStory`.

- [ ] **Step 6: Run checks**

Run:

```bash
cargo test -p gpui-component theme::schema::tests::default_light_and_classic_dark_define_required_ui_tokens -- --nocapture
cargo check -p gpui-component
cargo check -p gpui-component-story
```

Expected: all commands exit 0.

- [ ] **Step 7: Commit Task 2**

```bash
git add crates/ui/src/theme/schema.rs crates/ui/src/theme/default-theme.json crates/story/src/stories/design_system_story.rs crates/story/src/stories/mod.rs crates/story/src/main.rs
git commit -m "refactor(ui): tune primary themes and story matrix"
```

---

## Task 3: App Workbench Shell Helpers

**Files:**
- Create: `crates/one_ui/src/workbench.rs`
- Modify: `crates/one_ui/src/lib.rs`

- [ ] **Step 1: Add workbench helper module**

Create `crates/one_ui/src/workbench.rs`:

```rust
use gpui::{App, Div, ParentElement as _, Styled as _, div, px};
use gpui_component::{ActiveTheme, h_flex, v_flex};

pub const RESOURCE_PANEL_WIDTH: f32 = 260.0;
pub const CONTEXT_PANEL_WIDTH: f32 = 360.0;
pub const WORKBENCH_TOOLBAR_HEIGHT: f32 = 40.0;

pub fn workbench_root(cx: &App) -> Div {
    h_flex()
        .size_full()
        .min_w_0()
        .min_h_0()
        .bg(cx.theme().background)
        .text_color(cx.theme().foreground)
}

pub fn resource_panel(cx: &App) -> Div {
    v_flex()
        .w(px(RESOURCE_PANEL_WIDTH))
        .h_full()
        .min_w(px(180.0))
        .flex_shrink_0()
        .border_r_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().sidebar)
}

pub fn main_panel(cx: &App) -> Div {
    v_flex()
        .flex_1()
        .size_full()
        .min_w_0()
        .min_h_0()
        .bg(cx.theme().background)
}

pub fn context_panel(cx: &App) -> Div {
    v_flex()
        .w(px(CONTEXT_PANEL_WIDTH))
        .h_full()
        .min_w(px(280.0))
        .flex_shrink_0()
        .border_l_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
}

pub fn workbench_toolbar(cx: &App) -> Div {
    h_flex()
        .h(px(WORKBENCH_TOOLBAR_HEIGHT))
        .min_h(px(WORKBENCH_TOOLBAR_HEIGHT))
        .w_full()
        .items_center()
        .gap_2()
        .px_3()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().muted)
}

pub fn split_border(cx: &App) -> Div {
    div().w(px(1.0)).h_full().bg(cx.theme().border)
}
```

- [ ] **Step 2: Export helpers**

Modify `crates/one_ui/src/lib.rs`:

```rust
pub mod workbench;
```

- [ ] **Step 3: Run crate check**

Run:

```bash
cargo check -p one-ui
```

Expected: exit 0.

- [ ] **Step 4: Commit Task 3**

```bash
git add crates/one_ui/src/workbench.rs crates/one_ui/src/lib.rs
git commit -m "refactor(one-ui): add workbench shell helpers"
```

---

## Task 4: Connection Display Helpers With Tests

**Files:**
- Create: `main/src/home/connection_display.rs`
- Modify: `main/src/home/mod.rs`
- Modify: `main/src/home_tab.rs`

- [ ] **Step 1: Create display helper tests first**

Create `main/src/home/connection_display.rs` with tests at the bottom:

```rust
use gpui_component::{Icon, IconName};
use one_core::storage::{ConnectionType, DatabaseType, RedisMode, StoredConnection};

pub(crate) fn connection_subtitle(conn: &StoredConnection) -> Option<String> {
    match conn.connection_type {
        ConnectionType::Database => conn.to_db_connection().ok().map(|params| {
            if matches!(params.database_type, DatabaseType::SQLite | DatabaseType::DuckDB) {
                params.host
            } else {
                let database = params
                    .database
                    .map(|database| format!("/{database}"))
                    .unwrap_or_default();
                format!("{}@{}:{}{}", params.username, params.host, params.port, database)
            }
        }),
        ConnectionType::SshSftp => conn
            .to_ssh_params()
            .ok()
            .map(|params| format!("{}@{}:{}", params.username, params.host, params.port)),
        ConnectionType::Redis => conn.to_redis_params().ok().map(|params| match params.mode {
            RedisMode::Standalone => format!("{}:{}/{}", params.host, params.port, params.db_index),
            RedisMode::Sentinel => {
                let (master_name, sentinel_count) = params
                    .sentinel
                    .as_ref()
                    .map(|sentinel| (sentinel.master_name.as_str(), sentinel.sentinels.len()))
                    .unwrap_or(("sentinel", 0));
                format!("{master_name} (sentinel:{sentinel_count})")
            }
            RedisMode::Cluster => {
                let node_count = params.cluster.as_ref().map(|cluster| cluster.nodes.len()).unwrap_or(0);
                format!("cluster ({node_count} nodes)")
            }
        }),
        ConnectionType::MongoDB => conn.to_mongodb_params().ok().map(|params| {
            if !params.host.is_empty() {
                params.port.map(|port| format!("{}:{port}", params.host)).unwrap_or(params.host)
            } else if !params.connection_string.is_empty() {
                params.connection_string
            } else {
                "MongoDB".to_string()
            }
        }),
        ConnectionType::Serial => conn.to_serial_params().ok().map(|params| {
            let parity = match params.parity {
                one_core::storage::models::SerialParity::None => 'N',
                one_core::storage::models::SerialParity::Odd => 'O',
                one_core::storage::models::SerialParity::Even => 'E',
            };
            format!(
                "{} ({}, {}{}{})",
                params.port_name, params.baud_rate, params.data_bits, parity, params.stop_bits
            )
        }),
        _ => None,
    }
}

pub(crate) fn connection_icon(conn: &StoredConnection) -> Icon {
    match conn.connection_type {
        ConnectionType::Database => conn
            .to_db_connection()
            .map(|params| params.database_type.as_icon())
            .unwrap_or_else(|_| IconName::Database.into()),
        ConnectionType::SshSftp => IconName::TerminalColor.into(),
        ConnectionType::Redis => IconName::Redis.into(),
        ConnectionType::MongoDB => IconName::MongoDB.into(),
        ConnectionType::Serial => IconName::SerialPort.into(),
        _ => IconName::Server.into(),
    }
}

pub(crate) fn has_team_badge(conn: &StoredConnection) -> bool {
    conn.team_id.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(connection_type: ConnectionType, params: serde_json::Value) -> StoredConnection {
        StoredConnection {
            id: Some(1),
            name: "Sample".to_string(),
            connection_type,
            params: params.to_string(),
            sort_order: 0,
            workspace_id: None,
            selected_databases: None,
            remark: None,
            sync_enabled: true,
            cloud_id: None,
            last_synced_at: None,
            created_at: None,
            updated_at: None,
            team_id: None,
            owner_id: None,
        }
    }

    #[test]
    fn database_subtitle_includes_database_path() {
        let conn = stored(ConnectionType::Database, serde_json::json!({
            "database_type": "PostgreSQL",
            "host": "db.local",
            "port": 5432,
            "username": "alice",
            "password": "",
            "database": "app",
            "service_name": null,
            "sid": null,
            "extra_params": {}
        }));

        assert_eq!(
            Some("alice@db.local:5432/app".to_string()),
            connection_subtitle(&conn)
        );
    }

    #[test]
    fn ssh_subtitle_uses_user_host_port() {
        let conn = stored(ConnectionType::SshSftp, serde_json::json!({
            "host": "box.local",
            "port": 22,
            "username": "deploy",
            "auth_method": "Agent",
            "connect_timeout": null,
            "keepalive_interval": null,
            "keepalive_max": null,
            "default_directory": null,
            "init_script": null,
            "jump_server": null,
            "proxy": null
        }));

        assert_eq!(
            Some("deploy@box.local:22".to_string()),
            connection_subtitle(&conn)
        );
    }
}
```

- [ ] **Step 2: Run the focused main test**

Run:

```bash
cargo test -p main home::connection_display::tests::database_subtitle_includes_database_path -- --nocapture
```

Expected: FAIL before `main/src/home/mod.rs` exports the new module.

- [ ] **Step 3: Register the module**

Modify `main/src/home/mod.rs`:

```rust
pub(crate) mod connection_display;
```

- [ ] **Step 4: Expose `generate_duplicate_name` test coverage**

Move `generate_duplicate_name` from `main/src/home_tab.rs` to `main/src/home/connection_display.rs` and keep the same function body. Add this test:

```rust
#[test]
fn duplicate_name_uses_incrementing_suffix() {
    let existing = ["Prod (副本)", "Prod (副本 2)"]
        .into_iter()
        .map(String::from)
        .collect();

    assert_eq!("Prod (副本 3)", generate_duplicate_name("Prod", &existing));
}
```

Update `main/src/home_tab.rs` to import it:

```rust
use crate::home::connection_display::generate_duplicate_name;
```

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test -p main home::connection_display::tests -- --nocapture
```

Expected: exit 0.

- [ ] **Step 6: Commit Task 4**

```bash
git add main/src/home/connection_display.rs main/src/home/mod.rs main/src/home_tab.rs
git commit -m "refactor(home): extract connection display helpers"
```

---

## Task 5: Split Command Home Rendering Modules

**Files:**
- Create: `main/src/home/home_toolbar.rs`
- Create: `main/src/home/home_navigation.rs`
- Create: `main/src/home/home_content.rs`
- Create: `main/src/home/connection_card.rs`
- Modify: `main/src/home/mod.rs`
- Modify: `main/src/home/home_workspace_filter.rs`
- Modify: `main/src/home_tab.rs`
- Modify: `main/locales/main.yml`

- [ ] **Step 1: Register the rendering modules**

Modify `main/src/home/mod.rs`:

```rust
pub(crate) mod connection_card;
pub(crate) mod home_content;
pub(crate) mod home_navigation;
pub(crate) mod home_toolbar;
```

- [ ] **Step 2: Open privacy for rendering-only extraction**

In `main/src/home_tab.rs`, mark these fields and methods `pub(crate)` so the sibling Home modules can render without duplicating behavior:

```rust
pub(crate) selected_filter: ConnectionType,
pub(crate) search_input: Entity<InputState>,
pub(crate) search_query: Entity<String>,
pub(crate) selected_connection_id: Option<i64>,
pub(crate) cloud_error: Option<String>,
pub(crate) syncing: bool,
pub(crate) sync_requested: bool,
pub(crate) pending_conflicts: Vec<SyncConflict>,
pub(crate) current_user: Option<UserInfo>,
pub(crate) logging_in: bool,
pub(crate) auth_error: Option<String>,
```

Also make these methods `pub(crate)`:

```rust
pub(crate) fn cards_reorder_enabled(&self, cx: &App) -> bool
pub(crate) fn move_connection_card(&mut self, from_id: i64, workspace_id: Option<i64>, to_id: i64, cx: &mut Context<Self>)
pub(crate) fn refresh_local_home_data(&mut self, cx: &mut Context<Self>)
pub(crate) fn trigger_sync(&mut self, cx: &mut Context<Self>)
pub(crate) fn show_conflict_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>)
pub(crate) fn show_login_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>)
pub(crate) fn confirm_edit_connection(&mut self, conn_id: i64, conn_name: String, db_type: Option<DatabaseType>, window: &mut Window, cx: &mut Context<Self>)
pub(crate) fn duplicate_connection(&mut self, conn: StoredConnection, window: &mut Window, cx: &mut Context<Self>)
pub(crate) fn confirm_delete_connection(&mut self, conn_id: i64, conn_name: String, window: &mut Window, cx: &mut Context<Self>)
pub(crate) fn show_encryption_key_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>)
pub(crate) fn match_connection_type(&self, conn: &StoredConnection) -> bool
pub(crate) fn match_connection(&self, conn: &StoredConnection, query: &str) -> bool
```

- [ ] **Step 3: Extract toolbar rendering**

Create `main/src/home/home_toolbar.rs` and move the existing bodies of `render_toolbar`, `render_workspace_filter_popover`, and `ensure_workspace_filter_list` from `main/src/home_tab.rs` into:

```rust
use crate::home::home_workspace_filter::{WorkspaceFilterDelegate, show_workspace_dialog};
use crate::home_tab::HomePage;
use crate::license::{is_feature_enabled, show_upgrade_dialog};
use gpui::{
    AnyElement, App, Context, Entity, IntoElement, ParentElement as _, Styled as _, Window, px,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    chrome,
    h_flex,
    input::Input,
    list::{List, ListState},
    popover::Popover,
    v_flex,
};
use one_core::{crypto, license::Feature};
use rust_i18n::t;

impl HomePage {
    pub(crate) fn render_toolbar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        chrome::app_toolbar(cx)
            .child(self.render_primary_toolbar_actions(window, cx))
            .child(gpui::div().flex_1())
            .child(self.render_secondary_toolbar_actions(window, cx))
    }

    fn render_primary_toolbar_actions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity();
        let is_syncing = self.syncing;
        let is_logged_in = self.current_user.is_some();
        let has_sync_license = is_feature_enabled(Feature::CloudSync, cx);
        let has_master_key = crypto::has_master_key();
        let has_conflicts = !self.pending_conflicts.is_empty();
        let conflict_count = self.pending_conflicts.len();

        h_flex()
            .gap_2()
            .items_center()
            .child(
                Button::new("new-connect-button")
                    .icon(IconName::Plus)
                    .primary()
                    .label(t!("Home.new_connection"))
                    .tooltip(t!("Home.new_connection"))
                    .on_click(window.listener_for(&view, move |this, _, window, cx| {
                        this.show_new_connection_dialog(window, cx);
                    })),
            )
            .child(
                Button::new("sync-button")
                    .icon(if has_sync_license { IconName::Refresh } else { IconName::Key })
                    .label(if is_syncing {
                        t!("Home.syncing").to_string()
                    } else if !has_sync_license {
                        t!("License.upgrade_to_pro").to_string()
                    } else {
                        t!("Home.sync").to_string()
                    })
                    .ghost()
                    .disabled((!is_logged_in && has_sync_license) || is_syncing)
                    .tooltip(if !is_logged_in && has_sync_license {
                        t!("Home.cloud_need_login")
                    } else if !has_sync_license {
                        t!("License.pro_required")
                    } else {
                        t!("Home.sync_tooltip")
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if !has_sync_license {
                            show_upgrade_dialog(window, cx);
                        } else {
                            this.trigger_sync(cx);
                        }
                    })),
            )
            .when(has_conflicts, |this| {
                this.child(
                    Button::new("conflict-button")
                        .icon(IconName::TriangleAlert)
                        .label(format!("{conflict_count}"))
                        .ghost()
                        .text_color(cx.theme().warning)
                        .tooltip(t!("Home.conflict_tooltip", count = conflict_count))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.show_conflict_dialog(window, cx);
                        })),
                )
            })
            .child(
                Button::new("encryption-key-button")
                    .icon(IconName::Key)
                    .label(if has_master_key {
                        t!("Encryption.key_unlocked").to_string()
                    } else {
                        t!("Encryption.edit_repo_password").to_string()
                    })
                    .ghost()
                    .when(has_master_key, |btn| btn.text_color(cx.theme().success))
                    .when(!has_master_key, |btn| btn.text_color(cx.theme().muted_foreground))
                    .tooltip(if has_master_key {
                        t!("Encryption.key_unlocked_tooltip")
                    } else {
                        t!("Encryption.key_locked_tooltip")
                    })
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_encryption_key_dialog(window, cx);
                    })),
            )
    }

    fn render_secondary_toolbar_actions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let workspace_filter_open = self.workspace_filter_open;
        let workspace_filter = self.render_workspace_filter_popover(workspace_filter_open, window, cx);

        h_flex()
            .gap_1()
            .items_center()
            .child(Input::new(&self.search_input).cleanable(true).w(px(260.0)))
            .child(
                Button::new("refresh-button")
                    .icon(IconName::Refresh)
                    .ghost()
                    .tooltip(t!("Home.refresh"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.refresh_local_home_data(cx);
                    })),
            )
            .child(workspace_filter)
    }
}
```

Keep the original popover/list bodies in this file, but replace local wrapper styling with `chrome::panel_surface(cx)` and `chrome::panel_header(cx)`.

- [ ] **Step 4: Extract global rail and connection navigation**

Create `main/src/home/home_navigation.rs`:

```rust
use crate::home_tab::HomePage;
use gpui::{Context, IntoElement, ParentElement as _, Styled as _, Window};
use gpui_component::{
    ActiveTheme, Icon, IconName, Selectable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    chrome, h_flex, v_flex,
};
use one_core::storage::ConnectionType;
use rust_i18n::t;

impl HomePage {
    pub(crate) fn render_home_navigation(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .h_full()
            .child(self.render_global_rail(window, cx))
            .child(self.render_connection_type_sidebar(window, cx))
    }

    fn render_global_rail(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        chrome::app_rail(cx)
            .child(Button::new("rail-home").icon(IconName::Home).ghost().selected(true).tooltip(t!("Home.title")))
            .child(Button::new("rail-ai").icon(IconName::AI).ghost().tooltip("ChatDB").on_click(cx.listener(|this, _, window, cx| {
                this.add_ai_chat_tab(window, cx);
            })))
            .child(gpui::div().flex_1())
            .child(Button::new("rail-settings").icon(IconName::Settings).ghost().tooltip(t!("Common.settings")).on_click(cx.listener(|this, _, window, cx| {
                this.add_settings_tab(window, cx);
            })))
    }

    fn render_connection_type_sidebar(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .w(gpui::px(184.0))
            .h_full()
            .flex_shrink_0()
            .gap_1()
            .p_2()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .children(ConnectionType::all().into_iter().filter(|kind| *kind != ConnectionType::ChatDB).map(|kind| {
                let selected = self.selected_filter == kind;
                Button::new(kind.label())
                    .ghost()
                    .selected(selected)
                    .w_full()
                    .justify_start()
                    .icon(Icon::new(kind.icon()).color())
                    .label(kind.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_filter = kind;
                        cx.notify();
                    }))
            }))
    }
}
```

Move the avatar/login control from the old sidebar into the bottom of `render_connection_type_sidebar` after the connection type list. Preserve the existing `render_user_avatar` call and login click behavior.

- [ ] **Step 5: Extract content and empty states**

Create `main/src/home/home_content.rs` and move `render_content_area`, `render_workspace_view`, `render_workspace_section`, `render_connections_grid`, and `render_unassigned_section` from `main/src/home_tab.rs`.

Add these empty-state helpers:

```rust
impl HomePage {
    fn render_home_empty_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        gpui_component::v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .text_color(cx.theme().muted_foreground)
            .child(gpui_component::Icon::new(gpui_component::IconName::Server).large())
            .child(gpui::div().text_sm().child(rust_i18n::t!("Home.empty_command_home").to_string()))
            .child(gpui::div().text_xs().child(rust_i18n::t!("Home.empty_command_home_hint").to_string()))
    }

    fn render_home_filtered_empty_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        gpui_component::v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .text_color(cx.theme().muted_foreground)
            .child(gpui_component::Icon::new(gpui_component::IconName::Search).large())
            .child(gpui::div().text_sm().child(rust_i18n::t!("Home.filtered_empty").to_string()))
            .child(gpui::div().text_xs().child(rust_i18n::t!("Home.filtered_empty_hint").to_string()))
    }
}
```

Inside `render_workspace_view`, render `render_home_empty_state` when `self.connections.is_empty()`, and render `render_home_filtered_empty_state` when all filtered groups are empty but there are existing connections.

- [ ] **Step 6: Extract connection card**

Create `main/src/home/connection_card.rs` and move `DragConnectionCard` plus `render_connection_card` from `main/src/home_tab.rs`.

Use `connection_subtitle`, `connection_icon`, and `has_team_badge` from `connection_display.rs`. Replace repeated per-type subtitle blocks with:

```rust
if let Some(conn_info) = crate::home::connection_display::connection_subtitle(&conn) {
    let tooltip_text: gpui::SharedString = conn_info.clone().into();
    this.child(
        gpui::div()
            .id(gpui::SharedString::from(format!("conn-info-{}", conn.id.unwrap_or(0))))
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .overflow_hidden()
            .text_ellipsis()
            .whitespace_nowrap()
            .max_w_full()
            .tooltip(move |window, cx| {
                gpui_component::tooltip::Tooltip::new(tooltip_text.clone()).build(window, cx)
            })
            .child(conn_info),
    )
} else {
    this
}
```

Keep these behaviors unchanged: single-click selection, double-click open with master-key guard, SFTP quick action, duplicate, edit, delete, drag reorder, active-dot display, team badge.

- [ ] **Step 7: Add locale strings**

Modify `main/locales/main.yml` under `Home`:

```yaml
  empty_command_home:
    en: No connections yet
    zh-CN: 还没有连接
    zh-HK: 還沒有連線
  empty_command_home_hint:
    en: Create a connection to start using your workspace.
    zh-CN: 新建一个连接后即可开始使用工作区。
    zh-HK: 新建一個連線後即可開始使用工作區。
  filtered_empty:
    en: No matching connections
    zh-CN: 没有匹配的连接
    zh-HK: 沒有匹配的連線
  filtered_empty_hint:
    en: Change the search text, connection type, or workspace filter.
    zh-CN: 请调整搜索内容、连接类型或工作区筛选。
    zh-HK: 請調整搜尋內容、連線類型或工作區篩選。
```

- [ ] **Step 8: Replace `HomePage::render` layout**

In `main/src/home_tab.rs`, replace the old sidebar call:

```rust
.child(self.render_sidebar(window, cx))
```

with:

```rust
.child(self.render_home_navigation(window, cx))
```

Keep `render_sidebar` removed from `home_tab.rs` after the new module compiles.

- [ ] **Step 9: Run focused checks**

Run:

```bash
cargo fmt --check
cargo check -p main
cargo test -p main home::connection_display::tests -- --nocapture
```

Expected: all commands exit 0.

- [ ] **Step 10: Commit Task 5**

```bash
git add main/src/home main/src/home_tab.rs main/locales/main.yml
git commit -m "refactor(home): build command home shell"
```

---

## Task 6: Apply Workbench Chrome To Main Surfaces

**Files:**
- Modify: `crates/db_view/src/database_tab.rs`
- Modify: `crates/redis_view/src/redis_tab.rs`
- Modify: `crates/redis_view/src/sidebar.rs`
- Modify: `crates/mongodb_view/src/mongo_tab.rs`
- Modify: `crates/mongodb_view/src/sidebar.rs`
- Modify: `crates/terminal_view/src/view.rs`
- Modify: `crates/terminal_view/src/sidebar/mod.rs`
- Modify: `crates/sftp_view/src/file_list_panel.rs`
- Modify: `main/src/setting_tab.rs`
- Modify: `main/src/new_connection/connection_window.rs`
- Modify: `main/src/new_connection/form_page.rs`

- [ ] **Step 1: Align database workbench**

In `crates/db_view/src/database_tab.rs`, import:

```rust
use one_ui::workbench;
```

In `impl Render for DatabaseTabView`, keep the existing tree, tab container, sidebar entity, resizing handles, and event handling. Replace only outer container builders with:

```rust
workbench::workbench_root(cx)
    .track_focus(&self.focus_handle)
    .on_mouse_move(cx.listener(Self::on_mouse_move))
    .on_mouse_up(cx.listener(Self::on_mouse_up))
```

Wrap the tree area in `workbench::resource_panel(cx)`, the tab container in `workbench::main_panel(cx)`, and the AI/sidebar area in `workbench::context_panel(cx)`. Preserve existing size variables `tree_panel_size` and `sidebar_panel_size`.

- [ ] **Step 2: Align Redis and Mongo shells**

In `crates/redis_view/src/redis_tab.rs` and `crates/mongodb_view/src/mongo_tab.rs`, import:

```rust
use one_ui::workbench;
```

Use:

```rust
workbench::workbench_root(cx)
workbench::resource_panel(cx)
workbench::main_panel(cx)
```

Keep current tree selection, key/collection selection, command execution, and detail panel logic unchanged.

In `crates/redis_view/src/sidebar.rs` and `crates/mongodb_view/src/sidebar.rs`, replace ad hoc header rows with:

```rust
workbench::workbench_toolbar(cx)
```

Retain all existing buttons and callbacks inside the toolbar.

- [ ] **Step 3: Align terminal shell**

In `crates/terminal_view/src/view.rs`, import:

```rust
use one_ui::workbench;
```

Use `workbench::workbench_root(cx)` for the root split, `workbench::main_panel(cx)` for the terminal element, and `workbench::context_panel(cx)` for the side panel when visible. Keep terminal font, render cache, agent command handling, addon frames, and keyboard context unchanged.

In `crates/terminal_view/src/sidebar/mod.rs`, use `workbench::workbench_toolbar(cx)` for side panel headers and keep the existing panel switching events.

- [ ] **Step 4: Align SFTP file panel**

In `crates/sftp_view/src/file_list_panel.rs`, import:

```rust
use gpui_component::chrome;
```

Use `chrome::panel_header(cx)` for path/search controls and keep sorting, selection, context menu, hidden-file filtering, and path edit behavior unchanged.

- [ ] **Step 5: Align settings and new-connection windows**

In `main/src/setting_tab.rs`, `main/src/new_connection/connection_window.rs`, and `main/src/new_connection/form_page.rs`, replace top-level ad hoc header rows with `chrome::app_toolbar(cx)` or `chrome::panel_header(cx)` where the visual role matches. Do not change form validation, save events, popup sizing, or tab creation logic.

- [ ] **Step 6: Run feature crate checks**

Run:

```bash
cargo check -p db_view
cargo check -p redis_view
cargo check -p mongodb_view
cargo check -p terminal_view
cargo check -p sftp_view
cargo check -p main
```

Expected: all commands exit 0.

- [ ] **Step 7: Commit Task 6**

```bash
git add crates/db_view/src/database_tab.rs crates/redis_view/src/redis_tab.rs crates/redis_view/src/sidebar.rs crates/mongodb_view/src/mongo_tab.rs crates/mongodb_view/src/sidebar.rs crates/terminal_view/src/view.rs crates/terminal_view/src/sidebar/mod.rs crates/sftp_view/src/file_list_panel.rs main/src/setting_tab.rs main/src/new_connection
git commit -m "refactor(ui): align workbench chrome"
```

---

## Task 7: Full Verification And Visual Smoke

**Files:**
- No source edits unless verification finds a concrete defect.

- [ ] **Step 1: Run formatting**

Run:

```bash
cargo fmt --check
```

Expected: exit 0.

- [ ] **Step 2: Run required cargo checks**

Run:

```bash
cargo check -p gpui-component
cargo check -p one-ui
cargo check -p gpui-component-story
cargo check -p main
```

Expected: all commands exit 0.

- [ ] **Step 3: Run focused tests added by this plan**

Run:

```bash
cargo test -p gpui-component button::button::tests::test_button_disabled_styles_keep_alpha_bounded -- --nocapture
cargo test -p gpui-component button::button::tests::test_text_and_ghost_buttons_use_theme_transparent_background -- --nocapture
cargo test -p gpui-component theme::schema::tests::default_light_and_classic_dark_define_required_ui_tokens -- --nocapture
cargo test -p main home::connection_display::tests -- --nocapture
```

Expected: all commands exit 0.

- [ ] **Step 4: Story visual smoke**

Run:

```bash
cargo run -p gpui-component-story
```

Inspect `Design System`, `Button`, `Input`, `List`, `Sidebar`, `Table`, `Popover`, `Dialog`, and `Theme Colors` in both `Default Light` and `macOS Classic Dark`.

Expected observations:
- Buttons have readable disabled, hover, active, selected, and loading states.
- Toolbar and rail dimensions are stable.
- Table and list hover/selected colors are visible in both themes.
- Popover/dialog surfaces have clear borders and no text overlap.

- [ ] **Step 5: Main app visual smoke**

Run:

```bash
cargo run -p main
```

Inspect:

- Home with connections.
- Home with empty connection list.
- Home with search producing no result.
- Workspace filter popover.
- Syncing, conflict, locked-key, unlocked-key, logged-in, and logged-out toolbar states.
- Database workbench tree, main tabs, and AI/context panel.
- Redis and MongoDB sidebars.
- Terminal side panel.
- SFTP file list toolbar and table.
- Settings and new connection windows.

Expected observations:
- No text overlap.
- No hover or selected state changes layout size.
- Existing click, double-click, drag reorder, edit, duplicate, delete, SFTP open, refresh, sync, login, key unlock, and workspace filter behavior remains available.

- [ ] **Step 6: Final review before merge or PR**

Run:

```bash
git status --short
git log --oneline --max-count=8
```

Expected:
- Only intentional UI refactor files are modified.
- Commits correspond to the task boundaries above.

Then use `superpowers:requesting-code-review` for the staged branch review.
