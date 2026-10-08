//! gray-hoarder — archive every session transcript into a local,
//! content-addressed hoard.
//!
//! Behavior-level port of @moyai/pi-session-hoarder (Apache-2.0), reduced to
//! its core promise: on every `turn_end` notification the sidecar finds the
//! session transcript under `~/.gray/sessions/`, SHA-256s its bytes, stores
//! them once at `~/.gray/hoarder/objects/<sha>` (existing objects are
//! skipped), and appends `{ts,session_id,cwd,sha,bytes}` to
//! `~/.gray/hoarder/index.jsonl`.
//!
//! `/hoard list|show <sha>|path` inspects the hoard. The notification
//! handler never replies and swallows every error — archiving must never
//! block or break a turn.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn manifest() -> Value {
    json!({
        "name": "hoarder",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "1.1",
        "tools": [],
        "commands": ["/hoard"],
    })
}

// ── paths ─────────────────────────────────────────────────────

fn gray_home() -> PathBuf {
    std::env::var_os("GRAY_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".gray")))
        .unwrap_or_else(|| PathBuf::from(".gray"))
}

fn hoard_dir() -> PathBuf {
    gray_home().join("hoarder")
}

/// Locate the transcript for a session id: `<id>.jsonl` first, then any
/// `<id>*` match under the sessions dir.
fn find_transcript_in(dir: &Path, session_id: &str) -> Option<PathBuf> {
    let dir = dir.to_path_buf();
    let exact = dir.join(format!("{session_id}.jsonl"));
    if exact.is_file() {
        return Some(exact);
    }
    let rd = std::fs::read_dir(&dir).ok()?;
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "jsonl").unwrap_or(false))
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(session_id))
                .unwrap_or(false)
        })
}

// ── archiving ─────────────────────────────────────────────────

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

fn now_iso() -> String {
    // unix seconds → ISO-8601 UTC, civil-from-days (Howard Hinnant)
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let rem = secs % 86400;
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = secs / 86400 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    if m <= 2 {
        y += 1;
    }
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Archive the transcript for `session`. Every error is swallowed by the
/// caller — this returns a Result only to make the happy path readable.
fn archive_in(home: &Path, session_id: &str, cwd: &str) -> std::io::Result<()> {
    let hoard = home.join("hoarder");
    let Some(src) = find_transcript_in(&home.join("sessions"), session_id) else { return Ok(()) };
    let bytes = std::fs::read(&src)?;
    if bytes.is_empty() {
        return Ok(());
    }
    let sha = sha256_hex(&bytes);
    let objects = hoard.join("objects");
    std::fs::create_dir_all(&objects)?;
    let obj = objects.join(&sha);
    if !obj.exists() {
        // write-then-rename so a crash can't leave a truncated object
        let tmp = objects.join(format!(".{sha}.tmp"));
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &obj).or_else(|_| std::fs::write(&obj, &bytes))?;
    }
    let entry = json!({
        "ts": now_iso(),
        "session_id": session_id,
        "cwd": cwd,
        "sha": sha,
        "bytes": bytes.len(),
    });
    let mut idx = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(hoard.join("index.jsonl"))?;
    writeln!(idx, "{entry}")?;
    Ok(())
}

// ── /hoard ────────────────────────────────────────────────────

fn index_entries() -> Vec<Value> {
    let Ok(text) = std::fs::read_to_string(hoard_dir().join("index.jsonl")) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect()
}

fn hoard_list() -> String {
    let entries = index_entries();
    if entries.is_empty() {
        return "hoard is empty — nothing archived yet".into();
    }
    let mut lines = vec![format!("{} archived snapshot(s):", entries.len())];
    for e in entries.iter().rev().take(15) {
        let sha = e.get("sha").and_then(Value::as_str).unwrap_or("?");
        lines.push(format!(
            "  {}  {}  {}B  {}",
            e.get("ts").and_then(Value::as_str).unwrap_or("?"),
            &sha[..sha.len().min(12)],
            e.get("bytes").and_then(Value::as_u64).unwrap_or(0),
            e.get("cwd").and_then(Value::as_str).unwrap_or("?"),
        ));
    }
    if entries.len() > 15 {
        lines.push(format!("  … and {} more", entries.len() - 15));
    }
    lines.join("\n")
}

fn hoard_show(sha_prefix: &str) -> String {
    let objects = hoard_dir().join("objects");
    let matches: Vec<PathBuf> = std::fs::read_dir(&objects)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with(sha_prefix))
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();
    if matches.is_empty() {
        return format!("no object matching {sha_prefix}");
    }
    let obj = &matches[0];
    let sha = obj.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string();
    let mut out = format!("object {sha}\n{}", obj.display());
    for e in index_entries().iter().rev() {
        if e.get("sha").and_then(Value::as_str) == Some(sha.as_str()) {
            out.push_str(&format!(
                "\narchived {} — session {} in {} ({} bytes)",
                e.get("ts").and_then(Value::as_str).unwrap_or("?"),
                e.get("session_id").and_then(Value::as_str).unwrap_or("?"),
                e.get("cwd").and_then(Value::as_str).unwrap_or("?"),
                e.get("bytes").and_then(Value::as_u64).unwrap_or(0),
            ));
            break;
        }
    }
    if let Ok(text) = std::fs::read_to_string(obj) {
        let preview: String = text.chars().take(2000).collect();
        out.push_str(&format!("\n\n{preview}"));
        if text.len() > 2000 {
            out.push_str("\n[… truncated …]");
        }
    }
    out
}

fn run_command(argv: &[&str]) -> String {
    match argv.first().copied() {
        None | Some("list") => hoard_list(),
        Some("show") => match argv.get(1) {
            Some(sha) => hoard_show(sha),
            None => "usage: /hoard show <sha>".into(),
        },
        Some("path") => hoard_dir().display().to_string(),
        Some(other) => format!("unknown subcommand: {other} (list | show <sha> | path)"),
    }
}

// ── wire ──────────────────────────────────────────────────────

fn handle(req: &Value) -> (Option<Value>, bool) {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let Some(id) = id else {
        if method == "event/notify"
            && params.get("type").and_then(Value::as_str) == Some("turn_end")
        {
            let session = params.get("session").cloned().unwrap_or(Value::Null);
            let sid = session.get("id").and_then(Value::as_str).unwrap_or("");
            let cwd = session.get("cwd").and_then(Value::as_str).unwrap_or("");
            if !sid.is_empty() {
                let _ = archive_in(&gray_home(), sid, cwd); // never block, never propagate
            }
        }
        return (None, method == "plugin/shutdown");
    };
    let result = match method {
        "plugin/manifest" => manifest(),
        "command/run" => {
            let argv: Vec<&str> = params
                .get("argv")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            json!({ "text": run_command(&argv) })
        }
        "plugin/shutdown" => return (Some(json!({ "id": id, "result": {} })), true),
        _ => {
            let error = json!({ "code": -32601, "message": "method not found" });
            return (Some(json!({ "id": id, "error": error })), false);
        }
    };
    (Some(json!({ "id": id, "result": result })), false)
}

fn main() -> std::io::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("manifest") {
        println!("{}", manifest());
        return Ok(());
    }
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(req) = serde_json::from_str::<Value>(&line) else { continue };
        let (reply, exit) = handle(&req);
        if let Some(reply) = reply {
            writeln!(stdout, "{reply}")?;
            stdout.flush()?;
        }
        if exit {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(method: &str, params: Value) -> Value {
        handle(&json!({ "id": 1, "method": method, "params": params })).0.unwrap()
    }

    #[test]
    fn manifest_has_one_command_no_hooks() {
        let m = call("plugin/manifest", Value::Null)["result"].clone();
        assert_eq!(m["name"], "hoarder");
        assert_eq!(m["commands"], json!(["/hoard"]));
        assert!(m.get("hooks").is_none());
    }

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn archive_stores_object_and_index() {
        let home = std::env::temp_dir().join(format!("grayhoard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("sessions")).unwrap();
        std::fs::write(home.join("sessions/s1.jsonl"), b"{\"a\":1}\n{\"b\":2}\n").unwrap();
        archive_in(&home, "s1", "/tmp/proj").unwrap();
        let sha = sha256_hex(b"{\"a\":1}\n{\"b\":2}\n");
        assert!(home.join(format!("hoarder/objects/{sha}")).is_file());
        let idx = std::fs::read_to_string(home.join("hoarder/index.jsonl")).unwrap();
        assert!(idx.contains("\"session_id\":\"s1\"") && idx.contains(&sha));
        // second run: dedupes the object, still appends an index entry
        archive_in(&home, "s1", "/tmp/proj").unwrap();
        let idx2 = std::fs::read_to_string(home.join("hoarder/index.jsonl")).unwrap();
        assert_eq!(idx2.lines().count(), 2);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn missing_session_is_silent_ok() {
        let home = std::env::temp_dir().join(format!("grayhoard-miss-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        archive_in(&home, "nonexistent", "/x").unwrap();
        let (reply, _exit) = handle(&json!({
            "method": "event/notify",
            "params": {"type": "turn_end", "session": {"id": "nonexistent", "cwd": "/x"}}
        }));
        assert!(reply.is_none());
    }

    #[test]
    fn hoard_path_and_list() {
        let r = call("command/run", json!({ "name": "/hoard", "argv": ["path"] }));
        assert!(r["result"]["text"].as_str().unwrap().contains("hoarder"));
    }

    #[test]
    fn shutdown_replies_then_exits() {
        let (reply, exit) = handle(&json!({ "id": 2, "method": "plugin/shutdown" }));
        assert!(reply.is_some() && exit);
    }
}
