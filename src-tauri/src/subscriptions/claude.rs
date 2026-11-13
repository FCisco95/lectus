//! The installed, unmodified official CLI owns Claude credentials. Every Lectus
//! login has its own config directory; no ~/.claude session is read or modified.
use super::*;
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{atomic::AtomicBool, Arc},
    thread,
};

fn binary() -> Result<PathBuf> {
    // Native executable only on Windows: cmd shims introduce an extra shell and
    // cannot provide reliable process-tree containment or arbitrary Unicode args.
    let mut dirs = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default();
    if let Some(home) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
        dirs.push(PathBuf::from(home).join(".local").join("bin"));
    }
    for dir in dirs {
        let path = dir.join(if cfg!(windows) {
            "claude.exe"
        } else {
            "claude"
        });
        if path.is_file() {
            return Ok(path);
        }
        // Official npm distributions also package the native executable.
        // Bypass their generated shell shim and contain the executable directly.
        let packaged = dir
            .join("node_modules")
            .join("@anthropic-ai")
            .join("claude-code")
            .join("bin")
            .join("claude.exe");
        if cfg!(windows) && packaged.is_file() {
            return Ok(packaged);
        }
    }
    bail!("Official Claude Code native CLI is unavailable. Install the official native Claude Code CLI, then connect it here. Lectus does not install or reuse global logins.");
}
fn active() -> Result<Option<PathBuf>> {
    let root = root()?;
    let pointer = root.join("claude-active");
    let name = match fs::read_to_string(pointer) {
        Ok(name) => name,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => bail!("Cannot read Lectus Claude connection."),
    };
    if name.is_empty()
        || name.len() > 100
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!("Lectus Claude connection needs a fresh sign-in.");
    }
    let path = root.join("claude").join(name);
    if !path.is_dir() {
        bail!("Lectus Claude connection needs a fresh sign-in.");
    }
    Ok(Some(path))
}
fn command(path: &Path) -> Result<Command> {
    super::claude_policy::ensure_unmanaged(path)?;
    let mut cmd = Command::new(binary()?);
    cmd.env_clear();
    // Allowlist OS essentials only. No ANTHROPIC_*, CLAUDE_*, cloud provider,
    // NODE_OPTIONS, API helpers, parent agent configuration or payment fallback.
    for name in [
        "PATH",
        "SystemRoot",
        "WINDIR",
        "SYSTEMDRIVE",
        "COMSPEC",
        "PATHEXT",
        "TEMP",
        "TMP",
        "HOME",
        "USERPROFILE",
        "LOCALAPPDATA",
        "APPDATA",
        "LANG",
        "LC_ALL",
        "TERM",
    ] {
        if let Some(value) = std::env::var_os(name) {
            cmd.env(name, value);
        }
    }
    let work = path.join("work");
    fs::create_dir_all(&work)
        .map_err(|_| anyhow!("Cannot create isolated Claude working directory."))?;
    cmd.env("CLAUDE_CONFIG_DIR", path)
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .current_dir(work)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    Ok(cmd)
}

#[cfg(windows)]
struct ProcessTree(windows::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl ProcessTree {
    fn attach(child: &std::process::Child) -> Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::{
            Foundation::{CloseHandle, HANDLE},
            System::JobObjects::*,
        };
        unsafe {
            let job = CreateJobObjectW(None, None)
                .map_err(|_| anyhow!("Cannot isolate Claude process tree."))?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of_val(&info) as u32,
            )
            .is_err()
                || AssignProcessToJobObject(job, HANDLE(child.as_raw_handle())).is_err()
            {
                let _ = CloseHandle(job);
                bail!("Cannot isolate Claude process tree. No text was sent.");
            }
            Ok(Self(job))
        }
    }
    fn kill(&self) {
        unsafe {
            let _ = windows::Win32::System::JobObjects::TerminateJobObject(self.0, 1);
        }
    }
}
#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
#[cfg(unix)]
struct ProcessTree(i32);
#[cfg(unix)]
impl ProcessTree {
    fn attach(child: &std::process::Child) -> Result<Self> {
        Ok(Self(child.id() as i32))
    }
    fn kill(&self) {
        unsafe {
            libc::kill(-self.0, libc::SIGKILL);
        }
    }
}

fn run(
    mut cmd: Command,
    input: Vec<u8>,
    lease: Lease,
    timeout: Duration,
    allow_failure: bool,
) -> Result<Vec<u8>> {
    lease.check()?;
    let mut child = cmd
        .spawn()
        .map_err(|_| anyhow!("Cannot start official Claude Code CLI. Check its installation."))?;
    let tree = match ProcessTree::attach(&child) {
        Ok(tree) => tree,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Cannot read Claude output."))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("Cannot read Claude error output."))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("Cannot send Claude input."))?;
    let overflow = Arc::new(AtomicBool::new(false));
    let reader = |stream: Box<dyn Read + Send>, overflow: Arc<AtomicBool>| {
        thread::spawn(move || {
            let result = read_bounded(stream, 256 * 1024);
            if result.is_err() {
                overflow.store(true, Ordering::SeqCst);
            }
            result
        })
    };
    let out = reader(Box::new(stdout), overflow.clone());
    let err = reader(Box::new(stderr), overflow.clone());
    let writer = thread::spawn(move || {
        lease.dispatch_text(|| {
            stdin
                .write_all(&input)
                .map_err(|_| anyhow!("Cannot send text to Claude."))
        })
    });
    let start = Instant::now();
    let result = loop {
        if lease.check().is_err() {
            break Err(anyhow!("Claude operation was cancelled or disconnected."));
        }
        if start.elapsed() >= timeout {
            break Err(anyhow!("Claude operation timed out. Try again."));
        }
        if overflow.load(Ordering::SeqCst) {
            break Err(anyhow!(
                "Claude output exceeded the size limit or was interrupted."
            ));
        }
        match child.try_wait() {
            Ok(Some(exit)) if out.is_finished() && err.is_finished() && writer.is_finished() => {
                break if exit.success() || allow_failure {
                    Ok(())
                } else {
                    Err(anyhow!("Claude request failed. Check subscription authentication, usage and CLI version."))
                }
            }
            Ok(_) => thread::sleep(Duration::from_millis(25)),
            Err(_) => break Err(anyhow!("Cannot monitor Claude process.")),
        }
    };
    // Kill the whole owned tree, including inherited pipes, before joining readers.
    tree.kill();
    let _ = child.kill();
    let _ = child.wait();
    let output = out
        .join()
        .map_err(|_| anyhow!("Cannot read Claude output."))?;
    let _ = err.join(); // stderr is bounded and deliberately never exposed.
    let input_result = writer
        .join()
        .map_err(|_| anyhow!("Cannot send text to Claude."))?;
    result?;
    input_result?;
    lease.check()?;
    output
}
fn auth(path: &Path, lease: Lease) -> Result<Option<String>> {
    let mut cmd = command(path)?;
    cmd.args(["auth", "status"]);
    let output = run(cmd, Vec::new(), lease, Duration::from_secs(10), true)?;
    let value: Value = serde_json::from_slice(&output).map_err(|_| anyhow!("Claude CLI cannot report authentication safely. Update the official CLI and sign in again."))?;
    if !protocol::claude_subscription(&value) {
        return Ok(None);
    }
    super::claude_policy::ensure_personal_subscription(&value)?;
    // Require modern auth status to attest CLAUDE_CONFIG_DIR isolation, rather
    // than accidentally accepting an older CLI's global Keychain login.
    let directory = value["configDirectory"].as_str().ok_or_else(|| anyhow!("Update official Claude Code to a version that reports configDirectory (2.1.268 or later)."))?;
    let expected = fs::canonicalize(path)
        .map_err(|_| anyhow!("Cannot validate Claude configuration directory."))?;
    let actual = fs::canonicalize(directory)
        .map_err(|_| anyhow!("Claude reported an invalid configuration directory."))?;
    if expected != actual {
        bail!("Claude configuration isolation was not honored. No text was sent.");
    }
    Ok(Some(
        value["email"]
            .as_str()
            .unwrap_or("Claude subscription")
            .to_owned(),
    ))
}
pub(super) fn connection(lease: Lease) -> Result<ConnectionStatus> {
    if let Err(e) = binary() {
        return Ok(status("claude", "unavailable", None, &e.to_string()));
    }
    let Some(path) = active()? else {
        return Ok(status(
            "claude",
            "disconnected",
            None,
            "Connect Claude with a separate Lectus subscription login.",
        ));
    };
    Ok(match auth(&path, lease)? { Some(label) => status("claude", "connected", Some(label), "Claude subscription authenticated. Run a connection test to verify inference."), None => status("claude", "disconnected", None, "Claude subscription authentication is required. API and third-party provider logins are not accepted.") })
}
pub(super) fn login(lease: Lease) -> Result<ConnectionStatus> {
    binary()?;
    let name = store::random()?;
    let path = root()?.join("claude").join(&name);
    fs::create_dir_all(&path).map_err(|_| anyhow!("Cannot prepare isolated Claude sign-in."))?;
    let mut cmd = command(&path)?;
    cmd.args(["auth", "login"]);
    run(cmd, Vec::new(), lease, Duration::from_secs(300), false)?;
    let label = auth(&path, lease)?.ok_or_else(|| anyhow!("Claude sign-in did not grant a personal Claude Pro/Max subscription; API billing is not used."))?;
    lease.commit(|| {
        // A single small pointer write selects an already authenticated directory.
        fs::write(root()?.join("claude-active"), &name)
            .map_err(|_| anyhow!("Cannot activate Lectus Claude sign-in."))
    })?;
    Ok(status(
        "claude",
        "connected",
        Some(label),
        "Claude subscription connected. Run a connection test to verify inference.",
    ))
}
fn authenticated_path(lease: Lease) -> Result<PathBuf> {
    let path = active()?
        .ok_or_else(|| anyhow!("Connect your Claude subscription in AI settings first."))?;
    if auth(&path, lease)?.is_none() {
        bail!("Claude subscription authentication is required. API billing and provider fallback are disabled.");
    }
    Ok(path)
}
pub(super) fn models(lease: Lease) -> Result<Vec<SubscriptionModel>> {
    authenticated_path(lease)?;
    Ok([
        ("", "Claude default (subscription)"),
        ("sonnet", "Claude Sonnet"),
        ("opus", "Claude Opus"),
        ("haiku", "Claude Haiku"),
    ]
    .into_iter()
    .map(|(id, name)| SubscriptionModel {
        id: id.into(),
        name: name.into(),
    })
    .collect())
}
fn generation_args(model: &str) -> Result<Vec<String>> {
    if !matches!(model, "" | "sonnet" | "opus" | "haiku") {
        bail!("Choose Claude default or an official Claude model alias in AI settings.");
    }
    let mut args = [
        "--safe-mode",
        "--restricted",
        "--print",
        "--output-format",
        "json",
        "--tools",
        "",
        "--disallowedTools",
        "*",
        "--strict-mcp-config",
        "--mcp-config",
        "{\"mcpServers\":{}}",
        "--setting-sources",
        "",
        "--settings",
        "{\"disableAllHooks\":true,\"autoMemoryEnabled\":false}",
        "--no-session-persistence",
        "--max-turns",
        "1",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    if !model.is_empty() {
        args.extend(["--model".into(), model.into()]);
    }
    Ok(args)
}
fn result_text(output: &[u8]) -> Result<String> {
    let value: Value =
        serde_json::from_slice(output).map_err(|_| anyhow!("Claude returned invalid output."))?;
    if value["type"] != "result"
        || value["subtype"] != "success"
        || value["is_error"] != false
        || value
            .get("stop_reason")
            .is_some_and(|v| v != "end_turn" && !v.is_null())
    {
        bail!("Claude did not complete the text request. Check subscription usage and try again.");
    }
    let text = value["result"]
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.len() <= 64 * 1024)
        .ok_or_else(|| anyhow!("Claude returned empty or oversized text."))?;
    Ok(text.to_owned())
}
pub(super) fn generate(
    lease: Lease,
    text: &str,
    instructions: &str,
    model: &str,
) -> Result<String> {
    let args = generation_args(model)?;
    let path = authenticated_path(lease)?;
    let mut cmd = command(&path)?;
    cmd.args(args).arg("--system-prompt").arg(instructions);
    // Transcript is stdin data, never argv, a shell command, or a file.
    let output = run(
        cmd,
        text.as_bytes().to_vec(),
        lease,
        Duration::from_secs(45),
        false,
    )?;
    // Recheck mode after request before releasing output, including a CLI that
    // silently switched to a non-subscription provider during execution.
    if auth(&path, lease)?.is_none() {
        bail!("Claude subscription authorization changed during the request.");
    }
    result_text(&output)
}
pub(super) fn disconnect(lease: Lease) -> Result<String> {
    let path = active()?;
    lease.commit(|| match fs::remove_file(root()?.join("claude-active")) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => bail!("Cannot clear Lectus Claude connection."),
    })?;
    if let Some(path) = path {
        let mut cmd = command(&path)?;
        cmd.args(["auth", "logout"]);
        if run(cmd, Vec::new(), lease, Duration::from_secs(10), false).is_err() {
            return Ok("Disconnected locally. Claude CLI logout was not confirmed; the isolated Lectus session is no longer used.".into());
        }
    }
    Ok("Claude disconnected from Lectus. Other Claude sessions were unchanged.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Explicit local installed-CLI probe; isolated temp config, no login or inference"]
    fn installed_cli_isolated_auth_probe() {
        let path = tempfile::tempdir().unwrap();
        let lease = Lease::current("claude").unwrap();
        let mut version = command(path.path()).unwrap();
        version.arg("--version");
        let version = run(version, Vec::new(), lease, Duration::from_secs(15), false).unwrap();
        let version = String::from_utf8(version).unwrap();
        assert!(version.contains("Claude Code"));
        let mut status_command = command(path.path()).unwrap();
        assert!(!status_command
            .get_envs()
            .any(|(key, _)| key.to_string_lossy().starts_with("ANTHROPIC_")));
        status_command.args(["--safe-mode", "--restricted", "auth", "status"]);
        let output = run(
            status_command,
            Vec::new(),
            lease,
            Duration::from_secs(15),
            true,
        )
        .unwrap();
        let value: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(
            value["loggedIn"], false,
            "Isolated directory must not reuse a global login"
        );
        assert_eq!(value["authMethod"], "none");
        let directory = value["configDirectory"]
            .as_str()
            .expect("CLI must attest configuration directory");
        assert_eq!(
            fs::canonicalize(directory).unwrap(),
            fs::canonicalize(path.path()).unwrap()
        );
        let mut flags = command(path.path()).unwrap();
        flags.args(generation_args("").unwrap()).arg("--help");
        let help = run(flags, Vec::new(), lease, Duration::from_secs(15), false).unwrap();
        assert!(String::from_utf8(help).unwrap().contains("--tools"));
        println!("Official native CLI: {}; unmanaged endpoint policy checks passed; safe-mode/restricted isolated authMethod=none; configDirectory matches; generation flag syntax accepted by --help; no login/inference.", version.trim());
    }
    #[test]
    fn cli_failure_truncation_and_refusal_never_return_partial_text() {
        assert_eq!(result_text(br#"{"type":"result","subtype":"success","is_error":false,"result":"final","stop_reason":"end_turn"}"#).unwrap(), "final");
        for output in [br#"{"result":"partial"}"#.as_slice(), br#"{"type":"result","subtype":"error_max_turns","is_error":true,"result":"partial"}"#, br#"{"type":"result","subtype":"success","is_error":false,"result":"partial","stop_reason":"refusal"}"#] { assert!(result_text(output).is_err()); }
    }
    #[test]
    fn generation_disables_tools_hooks_mcp_and_settings_discovery() {
        let args = generation_args("sonnet").unwrap();
        for pair in [
            ["--tools", ""],
            ["--disallowedTools", "*"],
            ["--setting-sources", ""],
            ["--mcp-config", "{\"mcpServers\":{}}"],
        ] {
            assert!(args
                .windows(2)
                .any(|window| window[0] == pair[0] && window[1] == pair[1]));
        }
        assert!(args.iter().any(|arg| arg == "--safe-mode"));
        assert!(!args.iter().any(|arg| arg == "--bare"));
        assert!(args.iter().any(|arg| arg == "--restricted"));
        assert!(generation_args("--dangerously-skip-permissions").is_err());
    }
}
