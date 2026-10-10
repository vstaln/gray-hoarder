<div align="center">
  <img alt="gray-hoarder" src="assets/icon.svg" width="120" height="120" />
  <h1>gray-hoarder</h1>
  <p><strong>Archive every session transcript into a local, content-addressed hoard.</strong></p>
  <p>
    <a href="https://gray.alignment.id">Website</a> ·
    <a href="https://gray.alignment.id/plugins/gray-hoarder">Store</a> ·
    <a href="https://github.com/vstaln/gray-hoarder">Source</a> ·
    <a href="https://github.com/vstaln/gray">gray</a>
  </p>
  <p>
    <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-1c1c20?style=flat-square&labelColor=0a0a0b" /></a>
    <a href="https://www.rust-lang.org"><img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-rust-1c1c20?style=flat-square&labelColor=0a0a0b&logo=rust&logoColor=d4a373" /></a>
    <a href="https://gray.alignment.id/plugins/gray-hoarder"><img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-1c1c20?style=flat-square&labelColor=0a0a0b&color=7aa2f7" /></a>
  </p>
</div>

<br/>

```bash
gray plugin install gray-hoarder
```

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

## Tags

`gray` `plugin` `hoarder` `rust`

---

Part of the [gray](https://github.com/vstaln/gray) plugin ecosystem —
the open-source AI agent harness. <https://gray.alignment.id>
