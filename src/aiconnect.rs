//! AI assistants (Setup > "Connect to AI assistants", File > Options > AI Assistants): adds Neramit
//! Project's MCP server (`NeramitPlanner.exe --mcp`) to the AI assistants on this PC, and takes it out
//! again (uninstall does too). Only our own entry is written or removed; a config file that cannot
//! be read is left alone.
//! - Claude Desktop: %APPDATA%\Claude\claude_desktop_config.json, `mcpServers`
//! - Codex: %USERPROFILE%\.codex\config.toml, `[mcp_servers.neramit-planner]`
//! - Claude Code: through its own command, `claude mcp add --scope user`

use std::path::{Path, PathBuf};

const SERVER: &str = "neramit-planner";
const TOML_TABLE: &str = "[mcp_servers.neramit-planner]";
const TOML_NOTE: &str = "# Neramit Planner (added by Neramit Planner Setup; removed on uninstall)";

fn claude_desktop_config() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("APPDATA")?).join("Claude");
    dir.is_dir().then(|| dir.join("claude_desktop_config.json"))
}

fn codex_config() -> Option<PathBuf> {
    let home = std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(|| std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join(".codex")))?;
    home.is_dir().then(|| home.join("config.toml"))
}

/// The Claude Code command line (claude.cmd / claude.exe on PATH).
fn claude_cli() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).flat_map(|d| ["claude.cmd", "claude.exe"].map(|f| d.join(f))).find(|p| p.is_file())
}

/// Claude Desktop's config with our server added; None when the file is not a JSON object.
pub fn json_with_server(text: &str, exe: &Path) -> Option<String> {
    let mut v: serde_json::Value = if text.trim().is_empty() { serde_json::json!({}) } else { serde_json::from_str(text).ok()? };
    let servers = v.as_object_mut()?.entry("mcpServers").or_insert_with(|| serde_json::json!({}));
    servers.as_object_mut()?.insert(SERVER.into(), serde_json::json!({ "command": exe.to_string_lossy(), "args": ["--mcp"] }));
    serde_json::to_string_pretty(&v).ok()
}

/// Claude Desktop's config without our server; None when there is nothing to change.
pub fn json_without_server(text: &str) -> Option<String> {
    let mut v: serde_json::Value = serde_json::from_str(text).ok()?;
    v.get_mut("mcpServers")?.as_object_mut()?.remove(SERVER)?;
    serde_json::to_string_pretty(&v).ok()
}

/// Codex's config.toml without our table (and the note above it).
pub fn toml_without_server(text: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut skipping = false;
    for line in text.lines() {
        let t = line.trim();
        if t == TOML_TABLE || t.starts_with("[mcp_servers.neramit-planner.") {
            skipping = true;
            if out.last().map(|l| l.trim()) == Some(TOML_NOTE) {
                out.pop();
            }
            continue;
        }
        if skipping && t.starts_with('[') {
            skipping = false;
        }
        if !skipping {
            out.push(line);
        }
    }
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    let mut s = out.join("\n");
    if !s.is_empty() {
        s.push('\n');
    }
    s
}

/// Codex's config.toml with our table (replacing an earlier one).
pub fn toml_with_server(text: &str, exe: &Path) -> String {
    let mut s = toml_without_server(text);
    if !s.is_empty() {
        s.push('\n');
    }
    let path = exe.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    s.push_str(&format!("{TOML_NOTE}\n{TOML_TABLE}\ncommand = \"{path}\"\nargs = [\"--mcp\"]\n"));
    s
}

fn write_atomic(path: &Path, text: &str) -> bool {
    let tmp = path.with_extension("neramit-tmp");
    std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, path).is_ok()
}

fn claude_cli_run(args: &[&str]) -> bool {
    let Some(cli) = claude_cli() else { return false };
    use std::os::windows::process::CommandExt;
    // (Rust runs a claude.cmd batch file itself, with its arguments escaped for it)
    std::process::Command::new(&cli)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// An AI assistant Neramit Planner can be connected to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assistant {
    ClaudeCode,
    ClaudeDesktop,
    Codex,
}

impl Assistant {
    pub const ALL: [Assistant; 3] = [Assistant::ClaudeCode, Assistant::ClaudeDesktop, Assistant::Codex];

    pub fn name(self) -> &'static str {
        match self {
            Assistant::ClaudeCode => "Claude Code",
            Assistant::ClaudeDesktop => "Claude Desktop",
            Assistant::Codex => "Codex",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// the assistant is not on this PC
    Missing,
    Off,
    On,
}

/// Claude Code's user settings, where `claude mcp add --scope user` keeps its servers.
fn claude_code_settings() -> Option<PathBuf> {
    match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(d) => Some(PathBuf::from(d).join(".claude.json")),
        None => std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join(".claude.json")),
    }
}

fn json_has_server(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .is_some_and(|v| v.get("mcpServers").and_then(|m| m.get(SERVER)).is_some())
}

pub fn state(a: Assistant) -> State {
    let on = match a {
        Assistant::ClaudeCode => {
            if claude_cli().is_none() {
                return State::Missing;
            }
            claude_code_settings().is_some_and(|p| json_has_server(&p))
        }
        Assistant::ClaudeDesktop => match claude_desktop_config() {
            None => return State::Missing,
            Some(cfg) => json_has_server(&cfg),
        },
        Assistant::Codex => match codex_config() {
            None => return State::Missing,
            Some(cfg) => std::fs::read_to_string(cfg).is_ok_and(|t| t.lines().any(|l| l.trim() == TOML_TABLE)),
        },
    };
    if on { State::On } else { State::Off }
}

/// Adds Neramit Planner to one assistant.
pub fn connect_one(a: Assistant, exe: &Path) -> bool {
    match a {
        Assistant::ClaudeDesktop => claude_desktop_config().is_some_and(|cfg| {
            let text = std::fs::read_to_string(&cfg).unwrap_or_default();
            json_with_server(&text, exe).is_some_and(|t| write_atomic(&cfg, &t))
        }),
        Assistant::Codex => codex_config().is_some_and(|cfg| {
            let text = std::fs::read_to_string(&cfg).unwrap_or_default();
            write_atomic(&cfg, &toml_with_server(&text, exe))
        }),
        Assistant::ClaudeCode => {
            if claude_cli().is_none() {
                return false;
            }
            // (add refuses a name that exists: an earlier entry goes first)
            claude_cli_run(&["mcp", "remove", "--scope", "user", SERVER]);
            claude_cli_run(&["mcp", "add", "--scope", "user", SERVER, "--", &exe.to_string_lossy(), "--mcp"])
        }
    }
}

/// Takes Neramit Planner out of one assistant (true when it is no longer there).
pub fn disconnect_one(a: Assistant) -> bool {
    match a {
        Assistant::ClaudeDesktop => match claude_desktop_config().and_then(|cfg| std::fs::read_to_string(&cfg).ok().map(|t| (cfg, t))) {
            Some((cfg, text)) => match json_without_server(&text) {
                Some(t) => write_atomic(&cfg, &t),
                None => true,
            },
            None => true,
        },
        Assistant::Codex => match codex_config().and_then(|cfg| std::fs::read_to_string(&cfg).ok().map(|t| (cfg, t))) {
            Some((cfg, text)) if text.lines().any(|l| l.trim() == TOML_TABLE) => write_atomic(&cfg, &toml_without_server(&text)),
            _ => true,
        },
        Assistant::ClaudeCode => claude_cli_run(&["mcp", "remove", "--scope", "user", SERVER]) || state(a) != State::On,
    }
}

/// Connects every assistant found; returns their names.
pub fn connect(exe: &Path) -> Vec<&'static str> {
    Assistant::ALL.into_iter().filter(|&a| state(a) != State::Missing && connect_one(a, exe)).map(Assistant::name).collect()
}

/// Takes Neramit Planner out of every assistant it was added to.
pub fn disconnect() {
    for a in Assistant::ALL {
        if state(a) == State::On {
            disconnect_one(a);
        }
    }
}

// ---- the Options page: states are read once and again after a change; changes run on a thread
// (Claude Code's own command takes a few seconds)

static STATES: std::sync::Mutex<Option<[State; 3]>> = std::sync::Mutex::new(None);
static BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// The last change: (assistant, connect asked, succeeded)
static LAST: std::sync::Mutex<Option<(Assistant, bool, bool)>> = std::sync::Mutex::new(None);

pub fn states() -> [State; 3] {
    let mut s = STATES.lock().unwrap_or_else(|e| e.into_inner());
    *s.get_or_insert_with(|| Assistant::ALL.map(state))
}

/// Read the states again (the page was opened: an assistant may have been installed or changed).
pub fn refresh() {
    *STATES.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *LAST.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

pub fn busy() -> bool {
    BUSY.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn last_result() -> Option<(Assistant, bool, bool)> {
    *LAST.lock().unwrap_or_else(|e| e.into_inner())
}

/// Connects or disconnects `a` in the background; `done` runs afterwards (a repaint).
pub fn set_in_background(a: Assistant, on: bool, done: impl FnOnce() + Send + 'static) {
    if BUSY.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    std::thread::spawn(move || {
        let exe = std::env::current_exe().unwrap_or_default();
        let ok = if on { connect_one(a, &exe) } else { disconnect_one(a) };
        let now = Assistant::ALL.map(state);
        *STATES.lock().unwrap_or_else(|e| e.into_inner()) = Some(now);
        crate::setup::remember_ai(now.contains(&State::On));
        *LAST.lock().unwrap_or_else(|e| e.into_inner()) = Some((a, on, ok));
        BUSY.store(false, std::sync::atomic::Ordering::Relaxed);
        done();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cargo test -- --ignored claude_cli_runs`: the Claude Code command starts without cmd /c
    /// (read-only: it only asks for the version)
    #[test]
    #[ignore]
    fn claude_cli_runs() {
        if claude_cli().is_some() {
            assert!(claude_cli_run(&["--version"]));
        }
    }

    #[test]
    fn claude_desktop_entry_is_added_and_removed_alone() {
        let exe = Path::new(r"C:\Users\a\AppData\Local\Programs\Neramit Planner\NeramitPlanner.exe");
        let before = r#"{"mcpServers":{"other":{"command":"x"}},"theme":"dark"}"#;
        let with = json_with_server(before, exe).unwrap();
        let v: serde_json::Value = serde_json::from_str(&with).unwrap();
        assert_eq!(v["mcpServers"]["neramit-planner"]["args"][0], "--mcp");
        assert_eq!(v["mcpServers"]["neramit-planner"]["command"], exe.to_string_lossy().as_ref());
        assert_eq!(v["mcpServers"]["other"]["command"], "x");
        let without: serde_json::Value = serde_json::from_str(&json_without_server(&with).unwrap()).unwrap();
        assert_eq!(without, serde_json::from_str::<serde_json::Value>(before).unwrap());
        assert!(json_with_server("not json", exe).is_none());
        assert!(json_with_server("", exe).is_some());
        assert!(json_without_server(before).is_none());
    }

    #[test]
    fn codex_table_is_added_once_and_removed_alone() {
        let exe = Path::new(r"C:\Programs\Neramit Planner\NeramitPlanner.exe");
        let before = "model = \"gpt\"\n\n[mcp_servers.other]\ncommand = \"x\"\n";
        let once = toml_with_server(before, exe);
        let twice = toml_with_server(&once, exe);
        assert_eq!(once, twice);
        assert!(once.contains("command = \"C:\\\\Programs\\\\Neramit Planner\\\\NeramitPlanner.exe\""));
        assert!(once.starts_with(before));
        assert_eq!(toml_without_server(&once), before);
        // a table after ours survives
        let mixed = format!("{once}\n[profiles.fast]\nmodel = \"y\"\n");
        let back = toml_without_server(&mixed);
        assert!(back.contains("[profiles.fast]") && back.contains("[mcp_servers.other]") && !back.contains("neramit-planner"));
        assert_eq!(toml_with_server("", exe).lines().next(), Some(TOML_NOTE));
    }
}
