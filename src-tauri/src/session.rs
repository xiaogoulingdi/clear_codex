use std::{
  collections::HashMap,
  ffi::OsStr,
  io::{Read, Write},
  path::{Path, PathBuf},
  process::Command,
  sync::Arc,
  thread,
};

use anyhow::{anyhow, Context};
use parking_lot::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::{
  models::{CloseMode, SessionMetadata, SessionOutputEvent, SessionStatus, SessionStatusEvent},
  AppState,
};

const DEFAULT_COLS: u16 = 160;
const DEFAULT_ROWS: u16 = 24;

struct SessionHandle {
  metadata: Mutex<SessionMetadata>,
  writer: Mutex<Box<dyn Write + Send>>,
  master: Mutex<Box<dyn MasterPty + Send>>,
}

#[derive(Default)]
pub struct SessionManager {
  sessions: Mutex<HashMap<String, Arc<SessionHandle>>>,
}

impl SessionManager {
  pub fn from_snapshot(snapshot: &[SessionMetadata]) -> Self {
    let sessions = snapshot
      .iter()
      .map(|item| {
        let mut stale = item.clone();
        stale.status = if stale.pid.is_some_and(process_is_alive) {
          SessionStatus::Detached
        } else {
          stale.pid = None;
          stale.exit_code = stale.exit_code.or(Some(-1));
          SessionStatus::Exited
        };
        stale.reconnectable = false;

        let id = stale.id.clone();
        let handle = Arc::new(SessionHandle {
          metadata: Mutex::new(stale),
          writer: Mutex::new(Box::new(std::io::sink())),
          master: Mutex::new(Box::new(DetachedPty)),
        });
        (id, handle)
      })
      .collect();

    Self {
      sessions: Mutex::new(sessions),
    }
  }

  pub fn list(&self) -> Vec<SessionMetadata> {
    let mut sessions = self
      .sessions
      .lock()
      .values()
      .map(|session| session.metadata.lock().clone())
      .collect::<Vec<_>>();
    sessions.sort_by(|left, right| left.title.cmp(&right.title));
    sessions
  }

  pub fn create_session(
    &self,
    app: &AppHandle,
    cwd: String,
    title: Option<String>,
  ) -> anyhow::Result<SessionMetadata> {
    let codex_command = resolve_codex_launcher()?;
    let cwd_path = Path::new(&cwd);
    if !cwd_path.exists() {
      return Err(anyhow!("working directory does not exist: {cwd}"));
    }

    let pty_system = native_pty_system();
    let pair = pty_system
      .openpty(PtySize {
        rows: DEFAULT_ROWS,
        cols: DEFAULT_COLS,
        pixel_width: 0,
        pixel_height: 0,
      })
      .context("failed to create ConPTY session")?;

    let mut command = CommandBuilder::new(&codex_command.program);
    for arg in &codex_command.args {
      command.arg(arg);
    }
    command.cwd(cwd_path);
    command.env("TERM", "xterm-256color");

    let mut child = pair
      .slave
      .spawn_command(command)
      .context("failed to spawn codex")?;
    drop(pair.slave);

    let session_id = Uuid::new_v4().to_string();
    let session_title = title.unwrap_or_else(|| format!("Codex {}", self.list().len() + 1));
    let pid = child.process_id();
    let metadata = SessionMetadata {
      id: session_id.clone(),
      title: session_title,
      cwd,
      shell: "ConPTY".into(),
      codex_command: codex_command.display,
      status: SessionStatus::Running,
      persist_on_close: true,
      pid,
      exit_code: None,
      reconnectable: true,
    };

    let writer = pair
      .master
      .take_writer()
      .context("failed to capture terminal writer")?;
    let mut reader = pair
      .master
      .try_clone_reader()
      .context("failed to capture terminal reader")?;

    let handle = Arc::new(SessionHandle {
      metadata: Mutex::new(metadata.clone()),
      writer: Mutex::new(writer),
      master: Mutex::new(pair.master),
    });
    self
      .sessions
      .lock()
      .insert(session_id.clone(), Arc::clone(&handle));

    let read_app = app.clone();
    let read_session_id = session_id.clone();
    thread::spawn(move || {
      let mut buffer = [0_u8; 8192];
      loop {
        match reader.read(&mut buffer) {
          Ok(0) => break,
          Ok(size) => {
            let data = String::from_utf8_lossy(&buffer[..size]).to_string();
            let _ = read_app.emit(
              "session-output",
              SessionOutputEvent {
                session_id: read_session_id.clone(),
                data,
              },
            );
          }
          Err(_) => break,
        }
      }
    });

    let wait_app = app.clone();
    let wait_session_id = session_id.clone();
    thread::spawn(move || {
      let exit_status = child.wait().ok();
      let exit_code = exit_status
        .as_ref()
        .map(|status| status.exit_code() as i32);
      if let Some(state) = wait_app.try_state::<AppState>() {
        state.session_manager.mark_exited(&wait_session_id, exit_code.unwrap_or(-1));
        let _ = state.persist();
      }

      let _ = wait_app.emit(
        "session-status",
        SessionStatusEvent {
          session_id: wait_session_id,
          status: SessionStatus::Exited,
          exit_code,
          message: None,
        },
      );
    });

    Ok(metadata)
  }

  pub fn send_input(&self, session_id: &str, data: &str) -> anyhow::Result<()> {
    let session = self.get_live_session(session_id)?;
    let mut writer = session.writer.lock();
    writer.write_all(data.as_bytes())?;
    writer.flush()?;
    Ok(())
  }

  pub fn resize_session(&self, session_id: &str, cols: u16, rows: u16) -> anyhow::Result<()> {
    let session = self.get_live_session(session_id)?;
    let master = session.master.lock();
    master.resize(PtySize {
      rows,
      cols,
      pixel_width: 0,
      pixel_height: 0,
    })?;
    Ok(())
  }

  pub fn attach_session(&self, session_id: &str) -> anyhow::Result<SessionMetadata> {
    let session = self.get_session(session_id)?;
    let mut metadata = session.metadata.lock();
    metadata.status = if metadata.reconnectable {
      SessionStatus::Running
    } else {
      SessionStatus::Detached
    };
    Ok(metadata.clone())
  }

  pub fn close_session(&self, session_id: &str, mode: CloseMode) -> anyhow::Result<SessionMetadata> {
    let session = self.get_session(session_id)?;
    let mut metadata = session.metadata.lock();

    match mode {
      CloseMode::Detach => {
        metadata.status = SessionStatus::Detached;
        metadata.reconnectable = false;
      }
      CloseMode::Terminate => {
        if let Some(pid) = metadata.pid {
          terminate_process(pid)?;
        }
        metadata.status = SessionStatus::Exited;
        metadata.exit_code = Some(-1);
        metadata.reconnectable = false;
      }
    }

    Ok(metadata.clone())
  }

  pub fn mark_exited(&self, session_id: &str, exit_code: i32) {
    if let Ok(session) = self.get_session(session_id) {
      let mut metadata = session.metadata.lock();
      metadata.status = SessionStatus::Exited;
      metadata.exit_code = Some(exit_code);
      metadata.reconnectable = false;
      metadata.pid = None;
    }
  }

  fn get_live_session(&self, session_id: &str) -> anyhow::Result<Arc<SessionHandle>> {
    let session = self.get_session(session_id)?;
    let metadata = session.metadata.lock();
    if metadata.status == SessionStatus::Detached || !metadata.reconnectable {
      return Err(anyhow!("session {session_id} is not currently attachable"));
    }
    drop(metadata);
    Ok(session)
  }

  fn get_session(&self, session_id: &str) -> anyhow::Result<Arc<SessionHandle>> {
    self
      .sessions
      .lock()
      .get(session_id)
      .cloned()
      .ok_or_else(|| anyhow!("session not found: {session_id}"))
  }
}

#[derive(Debug)]
struct DetachedPty;

impl MasterPty for DetachedPty {
  fn resize(&self, _size: PtySize) -> anyhow::Result<()> {
    Ok(())
  }

  fn get_size(&self) -> anyhow::Result<PtySize> {
    Ok(PtySize {
      rows: DEFAULT_ROWS,
      cols: DEFAULT_COLS,
      pixel_width: 0,
      pixel_height: 0,
    })
  }

  fn try_clone_reader(&self) -> anyhow::Result<Box<dyn Read + Send>> {
    Ok(Box::new(std::io::empty()))
  }

  fn take_writer(&self) -> anyhow::Result<Box<dyn Write + Send>> {
    Ok(Box::new(std::io::sink()))
  }
}

struct CodexLaunchSpec {
  program: String,
  args: Vec<String>,
  display: String,
}

fn resolve_codex_launcher() -> anyhow::Result<CodexLaunchSpec> {
  let output = Command::new("where")
    .arg("codex")
    .output()
    .context("failed to locate codex in PATH")?;

  if !output.status.success() {
    return Err(anyhow!("codex CLI is not installed or not available in PATH"));
  }

  let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
  let launcher = stdout
    .lines()
    .map(str::trim)
    .find(|line| !line.is_empty())
    .ok_or_else(|| anyhow!("codex CLI path could not be resolved"))?;
  let launcher_path = PathBuf::from(launcher);
  let extension = launcher_path
    .extension()
    .and_then(OsStr::to_str)
    .map(str::to_ascii_lowercase)
    .unwrap_or_default();

  if matches!(extension.as_str(), "cmd" | "bat" | "ps1") {
    Ok(CodexLaunchSpec {
      program: "cmd.exe".into(),
      args: vec![
        "/D".into(),
        "/K".into(),
        format!("\"{}\"", launcher_path.display()),
      ],
      display: launcher_path.display().to_string(),
    })
  } else {
    Ok(CodexLaunchSpec {
      program: launcher_path.display().to_string(),
      args: Vec::new(),
      display: launcher_path.display().to_string(),
    })
  }
}

fn terminate_process(pid: u32) -> anyhow::Result<()> {
  let status = Command::new("taskkill")
    .args(["/PID", &pid.to_string(), "/T", "/F"])
    .status()
    .with_context(|| format!("failed to terminate process {pid}"))?;

  if status.success() {
    Ok(())
  } else {
    Err(anyhow!("taskkill failed for process {pid}"))
  }
}

fn process_is_alive(pid: u32) -> bool {
  Command::new("tasklist")
    .args(["/FI", &format!("PID eq {pid}")])
    .output()
    .map(|output| {
      output.status.success()
        && String::from_utf8_lossy(&output.stdout).contains(&pid.to_string())
    })
    .unwrap_or(false)
}
