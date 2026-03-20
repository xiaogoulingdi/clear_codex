use std::{
  collections::HashMap,
  io::{Read, Write},
  path::Path,
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
    ensure_codex_exists()?;
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

    let mut command = CommandBuilder::new("codex");
    command.cwd(cwd_path);
    command.env("TERM", "xterm-256color");

    let child = pair
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
      codex_command: "codex".into(),
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
      let exit_code = child.wait().ok();
      if let Some(state) = wait_app.try_state::<AppState>() {
        state
          .session_manager
          .mark_exited(&wait_session_id, exit_code.unwrap_or(-1));
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
  fn resize(&self, _size: PtySize) -> std::io::Result<()> {
    Ok(())
  }

  fn get_size(&self) -> std::io::Result<PtySize> {
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

fn ensure_codex_exists() -> anyhow::Result<()> {
  let output = Command::new("where")
    .arg("codex")
    .output()
    .context("failed to locate codex in PATH")?;

  if output.status.success() {
    Ok(())
  } else {
    Err(anyhow!("codex CLI is not installed or not available in PATH"))
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
