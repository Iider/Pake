//! Optional Kimi Code web client integration.
//!
//! Building with `PAKE_KIMI_WEB=1` in the environment turns the app into a
//! desktop client for Kimi Code's local web UI: on startup the app makes sure
//! `kimi web` (the foreground REST/WebSocket server that also serves the web
//! UI) is listening, then loads the UI with the persistent bearer token in
//! the `#token=` fragment, the same way `kimi web` opens the browser. A
//! server spawned by the app is stopped when the app exits; a server that was
//! already running is left alone.

use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, Url, WebviewWindow};

/// Compile-time switch: `PAKE_KIMI_WEB=1 cargo build` enables the client.
pub const ENABLED: bool = option_env!("PAKE_KIMI_WEB").is_some();

/// Origin served by `kimi web` in Kimi builds; callers use it to validate
/// session deep links before opening secondary windows.
pub fn base_url() -> Option<Url> {
    if !ENABLED {
        return None;
    }
    Url::parse(&format!("http://127.0.0.1:{KIMI_WEB_PORT}/")).ok()
}

/// Default `kimi web` bind port (its CLI retries busy ports with +1).
const KIMI_WEB_PORT: u16 = 58627;
const READY_POLL_INTERVAL: Duration = Duration::from_millis(200);
const READY_TIMEOUT: Duration = Duration::from_secs(60);

/// Shared handle to the `kimi web` child process spawned by this app.
pub type SharedChild = Arc<Mutex<Option<Child>>>;

pub fn shared_child() -> SharedChild {
    Arc::new(Mutex::new(None))
}

/// Ensure `kimi web` is serving, then navigate the main window to the
/// authenticated web UI. No-op in builds without `PAKE_KIMI_WEB`.
pub fn start(window: WebviewWindow, child: SharedChild) {
    if !ENABLED {
        return;
    }

    tauri::async_runtime::spawn(async move {
        if !server_listening() {
            match spawn_kimi_web() {
                Ok(process) => {
                    *child.lock().unwrap() = Some(process);
                }
                Err(error) => {
                    eprintln!("[Pake] Failed to launch `kimi web`: {error}");
                }
            }
        }

        if !wait_until_ready().await {
            eprintln!("[Pake] `kimi web` is not listening after {READY_TIMEOUT:?}");
            return;
        }

        let token = read_server_token_with_retry(window.app_handle().clone()).await;
        if let Err(error) = window.navigate(authenticated_url(token)) {
            eprintln!("[Pake] Failed to open the Kimi web UI: {error}");
        }
    });
}

/// Stop the server child this app spawned (if any). Servers that were already
/// running before the app started are never touched.
pub fn shutdown(child: &SharedChild) {
    if !ENABLED {
        return;
    }
    if let Some(mut process) = child.lock().unwrap().take() {
        let _ = process.kill();
        let _ = process.wait();
    }
}

fn server_listening() -> bool {
    TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], KIMI_WEB_PORT)),
        Duration::from_millis(300),
    )
    .is_ok()
}

async fn wait_until_ready() -> bool {
    let started = Instant::now();
    while started.elapsed() < READY_TIMEOUT {
        if server_listening() {
            return true;
        }
        tokio::time::sleep(READY_POLL_INTERVAL).await;
    }
    false
}

fn spawn_kimi_web() -> std::io::Result<Child> {
    let mut command = Command::new(kimi_program());
    command
        .args(["web", "--no-open", "--port"])
        .arg(KIMI_WEB_PORT.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // Keep the console server's window from popping up next to the GUI app.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let child = command.spawn()?;

    // Safety net: ensure the child dies with this process even on a
    // force-kill or crash, where the RunEvent::Exit cleanup never runs.
    #[cfg(windows)]
    if let Err(error) = assign_kill_on_close_job(&child) {
        eprintln!("[Pake] Failed to assign `kimi web` to a kill-on-close job: {error}");
    }

    Ok(child)
}

/// On Windows, put the child in a Job Object with KILL_ON_JOB_CLOSE so the
/// OS reaps it when the parent exits — even on a crash or Task Manager kill,
/// where the Rust shutdown path never runs. The job handle is intentionally
/// leaked: a raw pointer has no Drop, so the handle stays open until the OS
/// reclaims it on process exit, which is exactly when we want the child killed.
#[cfg(windows)]
fn assign_kill_on_close_job(child: &Child) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job.is_null() {
        return Err(std::io::Error::last_os_error());
    }

    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    let ok = unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }

    let child_handle = child.as_raw_handle();
    let ok = unsafe { AssignProcessToJobObject(job, child_handle) };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}

/// Prefer the native install location (`~/.kimi-code/bin/kimi`); fall back to
/// a plain PATH lookup for npm/pnpm-style installs.
fn kimi_program() -> PathBuf {
    #[cfg(windows)]
    const BINARY: &str = "kimi.exe";
    #[cfg(not(windows))]
    const BINARY: &str = "kimi";

    if let Some(home) = kimi_home_dir() {
        let bundled = home.join("bin").join(BINARY);
        if bundled.exists() {
            return bundled;
        }
    }
    PathBuf::from(BINARY)
}

/// Kimi Code's data directory: `KIMI_CODE_HOME` when set, else `~/.kimi-code`.
fn kimi_home_dir() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("KIMI_CODE_HOME") {
        if !custom.trim().is_empty() {
            return Some(PathBuf::from(custom));
        }
    }
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .map(|home| home.join(".kimi-code"))
}

/// Web UI URL with the persistent bearer token in the `#token=` fragment.
/// The fragment never reaches the server (it stays client-side); the web UI
/// reads it from `location.hash` and persists it, matching `kimi web`'s own
/// browser-open behavior. A missing token yields the bare origin and the UI
/// shows its auth gate.
fn authenticated_url(token: Option<String>) -> Url {
    let mut url = Url::parse(&format!("http://127.0.0.1:{KIMI_WEB_PORT}/"))
        .expect("loopback origin must be a valid URL");
    if let Some(token) = token {
        url.set_fragment(Some(&format!("token={token}")));
    }
    url
}

/// The server writes `server.token` on first boot, so this runs only after
/// the port is listening. Mirrors `tryResolveServerToken` in the CLI.
fn read_server_token(app: &AppHandle) -> Option<String> {
    let home = kimi_home_dir().or_else(|| {
        app.path()
            .home_dir()
            .ok()
            .map(|dir| dir.join(".kimi-code"))
    })?;
    let token = std::fs::read_to_string(home.join("server.token")).ok()?;
    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// Retry the token read for a short window to handle a brand-new server that
/// listens before it writes the persistent token file.
async fn read_server_token_with_retry(app: AppHandle) -> Option<String> {
    for _ in 0..25 {
        if let Some(token) = read_server_token(&app) {
            return Some(token);
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    None
}
