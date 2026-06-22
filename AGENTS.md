# AGENTS.md — claude-launcher project guide

This file is the authoritative reference for AI agents working on this project. Read it before making changes.

---

## Project overview

**claude-launcher** is a Rust CLI tool that manages inference provider profiles for [Claude Code](https://docs.anthropic.com/claude-code). It injects profile env vars at launch time, so `~/.claude/settings.json` is never modified.

The tool has two interaction modes:
- **Non-interactive** (`list`, `launch <slug>`, `--print`): plain stdout, no terminal raw mode.
- **Interactive TUI**: full-screen ratatui application using crossterm alternate screen + raw mode.

---

## Tech stack

| Crate | Version | Role |
|-------|---------|------|
| `ratatui` | 0.29 | TUI rendering |
| `crossterm` | 0.28 | Terminal backend, raw mode, key events |
| `tui-textarea` | 0.7 | Text input widgets inside ratatui |
| `clap` | 4 (derive) | CLI argument parsing |
| `serde` + `serde_json` | 1 | JSON config serialization |
| `dirs` | 5 | Home directory resolution |

Rust edition: **2024**. Binary target: `src/main.rs`.

Build target directory is redirected to `$CARGO_TARGET_DIR` (`~/.cache/rust/target` on this machine). Run `cargo build` and look there for the binary, not in `./target/`.

---

## Source layout

```
src/
├── main.rs            Entry point: parse CLI, set overrides, dispatch to action or TUI
├── cli.rs             Clap structs (Cli, Command enum)
├── error.rs           AppError, ConfigCorruptError, ConfigAccessError
├── config.rs          Profile CRUD; load/save JSON at 0600; slugify_name, mask_secret
├── settings.rs        Workspace CRUD; settings.json load/save; runtime overrides via OnceLock
├── providers/
│   └── mod.rs         16 static ProviderDef entries; FieldType enum; field! macro; get_provider()
├── actions/
│   ├── mod.rs         Re-exports
│   ├── list.rs        Non-interactive profile listing (plain stdout)
│   └── launch.rs      check_claude_installed, build_command, launch_claude, launch_with_slug
└── tui/
    ├── mod.rs         App struct; run() / run_app() event loop; crossterm setup/teardown
    ├── theme.rs       ORANGE, DIM_COLOR; style helpers: dim(), orange(), selected_item()
    └── screens/
        ├── mod.rs     Screen enum (owns state structs); Action enum; render/handle_key routers
        ├── widgets.rs SelectList (filterable); render_banner, render_footer, render_status
        ├── main_menu.rs
        ├── launch.rs
        ├── add.rs
        ├── edit.rs
        ├── delete.rs
        ├── settings.rs
        └── help.rs
```

---

## Key data shapes

### Profiles file (default: `$XDG_CONFIG_HOME/claude-launcher/workspaces/default.json`, mode 0600)
```json
{
  "profiles": {
    "<slug>": {
      "name": "Human-readable label",
      "provider": "<provider-id>",
      "env": {
        "ANTHROPIC_BASE_URL": "https://...",
        "ANTHROPIC_AUTH_TOKEN": "sk-..."
      }
    }
  }
}
```
Slugs are produced by `slugify_name(name)` in `config.rs`: lowercase, non-alphanumeric → `-`, deduped, collision → append `-2`, `-3`, etc.

### Settings file (`$XDG_CONFIG_HOME/claude-launcher/settings.json`)
```json
{
  "activeWorkspace": "default",
  "workspaces": {
    "default": "$XDG_CONFIG_HOME/claude-launcher/workspaces/default.json",
    "work": "$XDG_CONFIG_HOME/claude-launcher/workspaces/work.json"
  },
  "lastLaunchedProfile": "openrouter-anthropic"
}
```
`serde(rename_all = "camelCase")` is applied to `RawSettings`. `~` in paths is expanded by `expand_path()` in `settings.rs`.

## Providers

Providers are **statically compiled in** `src/providers/mod.rs`. There is no runtime extensibility. Each `ProviderDef` has:
- `id`: used to look up a provider by slug and stored in profiles JSON
- `name`: displayed in the TUI picker
- `fields: &'static [ProviderField]`: the form fields shown when adding/editing profiles

`FieldType` variants:
- `Url` — rendered as a plain text area, no masking
- `Secret` — masked with `•` in tui-textarea
- `String` — plain text, optional model/path overrides

**To add a new provider**: add a new `ProviderDef` constant and append it to the `PROVIDERS` static slice. No other files need to change.

---

## TUI architecture

### Event loop (`tui/mod.rs`)
1. Enable raw mode + enter alternate screen
2. Loop: draw → poll event (100ms timeout) → dispatch key
3. On `Action::LaunchClaude`: **disable raw mode first**, then exec `claude`
4. On any exit path: always restore terminal (disable_raw_mode + LeaveAlternateScreen + show_cursor)

### Screen pattern
Each screen module (`screens/<name>.rs`) exposes:
- A state struct (e.g. `AddState`, `EditState`) — owns all mutable UI state
- `pub fn render(f: &mut Frame, state: &mut <State>)` — pure rendering
- `pub fn handle_key(state: &mut <State>, key: KeyEvent) -> Nav` — returns a `Nav` enum
- A `Nav` enum with variants like `None`, `Back`, `Launch { .. }`, `AddProfile`

The central router in `screens/mod.rs` translates `Nav` → `Action` and updates `app.screen` by replacing it with a new state struct. `app.refresh_workspace()` is called on every `Back` to pick up settings changes.

### SelectList widget (`screens/widgets.rs`)
Filterable, scrollable list used across multiple screens.
- `filter: String` — substring filter (case-insensitive)
- `filter_items()` — rebuilds `filtered: Vec<(usize, String, String)>` (original_index, label, desc)
- `selected_original_index()` → `Option<usize>` — maps filtered selection back to the original items vec
- `render(f, area, show_filter)` — draws the list with optional filter indicator

### Conditional menu items (`main_menu.rs`)
`MainMenuState` has an `actions: Vec<&'static str>` parallel to `list.items`. `new()` uses `filter_map` to build items conditionally (e.g. hide "Launch Last" when no profile has been launched), collecting action slugs in `actions`. `handle_key` indexes into `state.actions[idx]` — never into the static `MENU_ITEMS` array, because filtered items have different indices.

### tui-textarea usage
- `TextArea<'static>` — borrow lifetime must be `'static` for storage in state structs
- To render: `f.render_widget(&*textarea, area)` (deref to get the widget impl)
- To mask: `textarea.set_mask_char('\u{2022}')`
- To forward key events: `textarea.input(key_event)`
- To read value: `textarea.lines().join("").trim().to_string()`
- Apply styling via `textarea.set_block(block)` and `textarea.set_cursor_style(style)`

---

## CLI flags

| Flag | Description |
|------|-------------|
| `--workspace <LABEL>` | Use a saved workspace for this run only |
| `--config <PATH>` | Ad-hoc profiles path, no workspace needed |
| `--profiles <SLUG>` | Legacy direct-launch flag |
| `--print` | Print env vars + command instead of launching |
| `-- <args>` | Pass-through args forwarded verbatim to `claude` |

`--workspace` and `--config` are mutually exclusive. They are stored in a `OnceLock<Mutex<Option<String>>>` in `settings.rs` and read by `get_config_path()` on every profiles access.

---

## Build & run

```bash
cargo build                            # dev build
cargo build --release                  # release build
cargo build 2>&1 | grep -E "^error"   # check for errors

# Binary location (CARGO_TARGET_DIR is set globally on this machine):
$CARGO_TARGET_DIR/debug/claude-launcher
$CARGO_TARGET_DIR/release/claude-launcher

# Smoke test
$CARGO_TARGET_DIR/debug/claude-launcher --version
$CARGO_TARGET_DIR/debug/claude-launcher list
$CARGO_TARGET_DIR/debug/claude-launcher --help
```

> **Note:** there is no `./target/` directory in the repo because `CARGO_TARGET_DIR=~/.cache/rust/target` is set in the shell environment. Always use `$CARGO_TARGET_DIR` to locate compiled artifacts.

---

## Testing

The `tests/` directory exists but contains no tests yet. The project has no test suite.

When adding tests, prefer integration tests in `tests/` that exercise CLI subcommands via `std::process::Command`. For config I/O tests, use `tempfile` to avoid polluting real config files.

---

## Common pitfalls

- **Terminal not restored on panic**: the `run()` function uses `ok()` to suppress teardown errors, but a panic in `run_app()` will leave the terminal in raw mode. Consider adding a panic hook if this becomes an issue.
- **`Action::LaunchClaude` timing**: raw mode MUST be disabled before spawning `claude`. The current implementation in `tui/mod.rs` does this correctly — don't move the teardown after the spawn.
- **Empty `slugify_name` result**: if a user enters a name that produces an empty slug (e.g. all symbols), `add.rs` and `edit.rs` show an error and refuse to save. Always check the slug before saving.
- **Profile file permissions**: `save_config()` sets 0600 on Unix via `fs::set_permissions`. On non-Unix this is a no-op — don't remove the `#[cfg(unix)]` guard.
- **Workspace label vs profile slug**: `slugify_label` (settings) and `slugify_name` (config) are different functions with the same logic. They're separate intentionally — workspace labels and profile slugs live in different namespaces.
- **`OnceLock` override order**: `set_runtime_config_path()` must be called **before** any profile access. `main.rs` does this immediately after parsing CLI args.
- **Workspace vs profile**: workspaces (settings.rs) are labels pointing to config file paths. Profiles (config.rs) are specific saved sets within a config file. They live in different namespaces. The "Launch Last" feature stores a profile slug, not a workspace label — confusing the two means the wrong thing gets launched.
