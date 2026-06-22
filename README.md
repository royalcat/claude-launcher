<div align="center">

# Claude Launcher

*Switch Claude Code providers instantly — no config edits, no friction.*

[![Rust](https://img.shields.io/badge/Rust-2024-f74c00?style=flat-square&logo=rust)](https://www.rust-lang.org)
[Why](#why) • [Run Claude Code for free](#run-claude-code-for-free) • [Quick start](#quick-start) • [Commands](#commands) • [Providers](#supported-providers)

![Demo](assets/demo--0.4.1.gif)

</div>

Run Claude Code with any Anthropic-compatible provider or API key in seconds. One command, zero config edits.

```bash
claude-launcher
```

## Why

Claude Code reads provider settings from `~/.claude/settings.json`. Switching providers normally means hand-editing that file — slow, error-prone, breaks flow.

Claude Launcher injects credentials at runtime instead:

- Environment variables override settings per-launch
- Your `settings.json` stays untouched
- Switch providers with a single command or pick from a menu

## Run Claude Code for free

Anthropic's hosted Claude Code requires a paid plan or API credits. Many third-party providers ship Anthropic-compatible endpoints with **free tiers or local-only inference**, so you can run Claude Code at zero cost.

| Provider | Free option |
|----------|-------------|
| **Ollama** | Fully local — no API key, no quota, Free cloud plan available for models like GPT-OSS 120B or QWEN-3.5 |
| **LM Studio / vLLM** | Self-hosted, runs on your hardware |
| **z.ai (GLM)** | Generous coding plan available |
| **OpenRouter** | Free models (`:free` suffix) |
| **DeepSeek / Qwen / Moonshot** | Generous free credits on signup |

> [!TIP]
> Pair a local Ollama setup with Claude Launcher to get an offline Claude Code workflow with no recurring cost.

## Features

- Instant provider switching from a menu or by slug
- Multiple credential sets per provider (work / personal / project)
- Zero changes to `~/.claude/settings.json`
- Works with cloud APIs and local runtimes (Ollama, LM Studio, vLLM)
- Credential file stored at `0600` permissions
- Args pass-through to `claude` for model overrides and flags
- Scriptable for CI pipelines via `--print`

## Quick start

```bash
# Interactive full-screen TUI (add, edit, launch, list, settings)
claude-launcher

# Pick credentials interactively, then launch claude
claude-launcher launch

# Launch directly with a saved credential slug
claude-launcher launch zai-personal

# Forward arguments to claude
claude-launcher launch zai-personal -- --model sonnet
```

## Commands

| Command | Description |
|---------|-------------|
| `claude-launcher` | Interactive full-screen TUI |
| `claude-launcher list` | List saved credentials |
| `claude-launcher launch` | Pick credentials interactively, then launch |
| `claude-launcher launch <slug>` | Launch with a specific credential |
| `claude-launcher launch <slug> --print` | Print env vars and command, don't spawn |
| `claude-launcher launch <slug> -- <args>` | Forward args verbatim to `claude` |

## Use cases
- Use Claude Code free without any subscription
- Compare providers side-by-side (OpenRouter vs DeepSeek vs z.ai)
- Switch between work and personal API keys
- Toggle between cheap and premium models per task
- Run Claude Code fully offline against Ollama or LM Studio
- Inject provider env vars in CI via `--print`

## Supported providers

Works with any Anthropic-compatible API. Built-in:

OpenRouter · DeepSeek · z.ai (GLM) · Ollama · LM Studio · vLLM · LiteLLM · Fireworks AI · Qwen · Moonshot · MiniMax · Volcengine · Cloudflare AI Gateway · Vercel AI Gateway · NVIDIA NIM

## Install

```bash
# From source (requires Rust toolchain)
git clone https://github.com/royalcat/claude-launcher.git
cd claude-launcher
cargo build --release
# Binary at: target/release/claude-launcher
```

Copy or symlink the binary to a directory on your `PATH`:

```bash
ln -sf "$(pwd)/target/release/claude-launcher" ~/.local/bin/claude-launcher
```

## Requirements

- Rust 1.85+ (edition 2024)
- Claude Code installed and on `PATH` (`claude --version` should work)

## Config

Two files:

- `$XDG_CONFIG_HOME/claude-launcher/settings.json` — settings (active profile + registered profile paths). Fixed location.
- `<credentials path>` — credentials JSON, mode `0600`, plaintext. Default: `$XDG_CONFIG_HOME/claude-launcher/providers.json`. Path is driven by the active profile.

> [!WARNING]
> The credentials file holds API keys in plaintext. Don't commit it, don't sync it to public cloud storage, and review backups before sharing.

### Profiles

A profile is a label paired with a credentials file path. Each profile holds its own set of credentials — switch contexts without retyping paths or editing files.

**Common setups:**

| Profile | Credentials file | Why |
|---------|-----------------|-----|
| `default` | `$XDG_CONFIG_HOME/claude-launcher/providers.json` | Personal keys, day-to-day use |
| `office` | `$XDG_CONFIG_HOME/claude-launcher/work.json` | Work API keys, separate billing |
| `cheap-models` | `$XDG_CONFIG_HOME/claude-launcher/cheap.json` | Free/low-cost providers only |
| `ci` | `/etc/claude-launcher-ci.json` | Read-only path injected in CI |

**Manage profiles interactively:**

```
claude-launcher → Settings → Manage profiles
```

From there you can add, rename, change path, delete, or switch the active profile.

**CLI overrides** — one-shot, never modify saved settings:

```bash
claude-launcher --profile office launch zai          # use "office" profile this run
claude-launcher --profile office list                # list credentials from "office" profile
claude-launcher --config /tmp/test.json list         # ad-hoc path, no profile needed
```

`--profile` and `--config` are mutually exclusive.

**How it works:**

- `$XDG_CONFIG_HOME/claude-launcher/settings.json` stores `{ activeProfile, profiles: { <label>: <path> } }`. Fixed location.
- Active profile's path drives which credentials file is read and written.
- On first run, a `default` profile pointing at `$XDG_CONFIG_HOME/claude-launcher/providers.json` is created automatically.
- Deleting the active profile is blocked — switch first, then delete.
- Profile labels are slugified (lowercase, non-alphanumerics → `-`).

**Tip:** use `--profile` in shell aliases to switch contexts without touching the active profile:

```bash
alias cc-work="claude-launcher --profile office launch"
alias cc-cheap="claude-launcher --profile cheap-models launch"
```

## Roadmap

- At-rest encryption for credentials files
