# gray-hoarder

Archive every session transcript into a local, content-addressed hoard.
A reduced, behavior-level port of
[@moyai/pi-session-hoarder](https://www.npmjs.com/package/@moyai/pi-session-hoarder)
(Apache-2.0).

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
