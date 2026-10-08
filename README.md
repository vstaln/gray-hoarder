<p align="center">
  <img src="assets/gray-logo.svg" alt="gray" width="96">
</p>
<h1 align="center">gray-hoarder</h1>
<p align="center">Archive every session transcript into a local, content-addressed hoard.</p>
<p align="center">
  <a href="https://github.com/vstaln/gray-hoarder/blob/main/LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-7aa2f7.svg">
  <img alt="rust" src="https://img.shields.io/badge/built%20with-rust-orange.svg">
</p>

Archive every session transcript into a local, content-addressed hoard.

A sidecar plugin for [gray](https://github.com/vstaln/gray), scaffolded by
[gray-account](https://github.com/vstaln/gray-account).

## What it does

On every `turn_end` notification, the sidecar:

1. Locates the session transcript under `~/.gray/sessions/`
   (`<session-id>.jsonl`, falling back to a `<session-id>*` glob).
2. SHA-256s the transcript bytes.
3. Stores them once at `~/.gray/hoarder/objects/<sha>` (write-then-rename;
   existing objects are skipped).
4. Appends `{ts, session_id, cwd, sha, bytes}` to
   `~/.gray/hoarder/index.jsonl`.

The notification handler never replies and swallows every error — hoarding
must never block or break a turn.

## Commands

- `/hoard` or `/hoard list` — recent archived snapshots (ts, sha, size, cwd)
- `/hoard show <sha>` — index entry plus a preview of the stored transcript
- `/hoard path` — print the hoard directory

State honors `$GRAY_HOME` first, then `$HOME/.gray`.

## Wire

`event/notify` (turn_end) + `command/run` for `/hoard`. Manifest claims
`commands: ["/hoard"]`, no tools, no hooks, protocol `1.1`.

## Install

```sh
gray plugin install hoarder
```

## Develop

```sh
cargo test
gray account check      # entry point + manifest handshake
gray account publish    # check → build → release → publish to the gray registry
```

Bump `version` in `Cargo.toml` before each `publish`; the registry refuses to
republish a version.

---

Part of the [gray](https://github.com/vstaln/gray) plugin ecosystem —
the open-source AI agent harness. <https://gray.alignment.id>
