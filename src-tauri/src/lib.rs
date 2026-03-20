mod models;
mod persistence;
mod session;

use anyhow::Context;
use models::{
  AppSnapshot, CloseMode, DockMode, OpacityMode, SessionMetadata, UiNoticeEvent, WindowState,
};
use parking_lot::Mutex;
use persistence::PersistenceStore;
use session::SessionManager;
use tauri::{
  Emitter,
  tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
  AppHandle, LogicalPosition, LogicalSize, Manager, State, WebviewWindow, WindowEvent,
};

const WINDOW_LABEL: &str = "main";
const DEFAULT_FOCUS_ALPHA: f64 = 0.88;
const DEFAULT_PEEK_ALPHA: f64 = 0.68;

pub struct AppState {
  active_session_id: Mutex<Option<String>>,
  notices: Mutex<Vec<UiNoticeEvent>>,
  persistence: PersistenceStore,
  session_manager: SessionManager,
  window_state: Mutex<WindowState>,
}

impl AppState {
  fn new(snapshot: AppSnapshot, persistence: PersistenceStore) -> Self {
    Self {
      active_session_id: Mutex::new(snapshot.active_session_id),
      notices: Mutex::new(Vec::new()),
      session_manager: SessionManager::from_snapshot(&snapshot.sessions),
      persistence,
      window_state: Mutex::new(snapshot.window),
    }
  }

  fn snapshot(&self) -> AppSnapshot {
    AppSnapshot {
      window: self.window_state.lock().clone(),
      sessions: self.session_manager.list(),
      active_session_id: self.active_session_id.lock().clone(),
    }
  }

  fn persist(&self) -> anyhow::Result<()> {
    self.persistence.save(&self.snapshot())
  }

  fn push_notice(&self, notice: UiNoticeEvent) {
    self.notices.lock().push(notice);
  }

  fn take_notices(&self) -> Vec<UiNoticeEvent> {
    let mut notices = self.notices.lock();
    let drained = notices.clone();
    notices.clear();
    drained
  }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionPayload {
  cwd: String,
  title: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendInputPayload {
  session_id: String,
  data: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResizePayload {
  session_id: String,
  cols: u16,
  rows: u16,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetActiveSessionPayload {
  session_id: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloseSessionPayload {
  session_id: String,
  mode: CloseMode,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowModePayload {
  opacity_mode: Option<OpacityMode>,
  click_through: Option<bool>,
  always_on_top: Option<bool>,
  dock_mode: Option<DockMode>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapPayload {
  snapshot: AppSnapshot,
  notices: Vec<UiNoticeEvent>,
  hotkey_summary: Vec<String>,
  default_cwd: String,
}

#[tauri::command]
fn bootstrap(state: State<'_, AppState>) -> Result<BootstrapPayload, String> {
  Ok(BootstrapPayload {
    snapshot: state.snapshot(),
    notices: state.take_notices(),
    hotkey_summary: vec![
      "Ctrl+Alt+Space: toggle overlay".into(),
      "Ctrl+Alt+N: new Codex tab".into(),
      "Ctrl+Alt+T: toggle click-through".into(),
      "Ctrl+Alt+Left / Ctrl+Alt+Right: switch tabs".into(),
    ],
    default_cwd: std::env::current_dir()
      .unwrap_or_else(|_| ".".into())
      .display()
      .to_string(),
  })
}

#[tauri::command]
fn create_session(
  app: AppHandle,
  state: State<'_, AppState>,
  payload: CreateSessionPayload,
) -> Result<SessionMetadata, String> {
  let session = state
    .session_manager
    .create_session(&app, payload.cwd, payload.title)
    .map_err(|error| error.to_string())?;

  *state.active_session_id.lock() = Some(session.id.clone());
  state.persist().map_err(|error| error.to_string())?;
  Ok(session)
}

#[tauri::command]
fn attach_session(
  state: State<'_, AppState>,
  session_id: String,
) -> Result<SessionMetadata, String> {
  let session = state
    .session_manager
    .attach_session(&session_id)
    .map_err(|error| error.to_string())?;

  *state.active_session_id.lock() = Some(session.id.clone());
  state.persist().map_err(|error| error.to_string())?;
  Ok(session)
}

#[tauri::command]
fn send_input(state: State<'_, AppState>, payload: SendInputPayload) -> Result<(), String> {
  state
    .session_manager
    .send_input(&payload.session_id, &payload.data)
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn resize_session(state: State<'_, AppState>, payload: ResizePayload) -> Result<(), String> {
  state
    .session_manager
    .resize_session(&payload.session_id, payload.cols, payload.rows)
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn close_session(
  state: State<'_, AppState>,
  payload: CloseSessionPayload,
) -> Result<SessionMetadata, String> {
  let session = state
    .session_manager
    .close_session(&payload.session_id, payload.mode)
    .map_err(|error| error.to_string())?;

  if state
    .active_session_id
    .lock()
    .as_ref()
    .is_some_and(|active| active == &session.id)
  {
    *state.active_session_id.lock() = state
      .session_manager
      .list()
      .into_iter()
      .find(|candidate| candidate.id != session.id && candidate.status != models::SessionStatus::Exited)
      .map(|candidate| candidate.id);
  }

  state.persist().map_err(|error| error.to_string())?;
  Ok(session)
}

#[tauri::command]
fn set_active_session(
  state: State<'_, AppState>,
  payload: SetActiveSessionPayload,
) -> Result<(), String> {
  *state.active_session_id.lock() = payload.session_id;
  state.persist().map_err(|error| error.to_string())
}

#[tauri::command]
fn update_window_mode(
  app: AppHandle,
  state: State<'_, AppState>,
  payload: WindowModePayload,
) -> Result<WindowState, String> {
  let window = app
    .get_webview_window(WINDOW_LABEL)
    .ok_or_else(|| "main window not found".to_string())?;

  {
    let mut current = state.window_state.lock();
    if let Some(opacity_mode) = payload.opacity_mode {
      current.opacity_mode = opacity_mode;
    }
    if let Some(click_through) = payload.click_through {
      current.click_through = click_through;
      window
        .set_ignore_cursor_events(click_through)
        .map_err(|error| error.to_string())?;
    }
    if let Some(always_on_top) = payload.always_on_top {
      current.always_on_top = always_on_top;
      window
        .set_always_on_top(always_on_top)
        .map_err(|error| error.to_string())?;
    }
    if let Some(dock_mode) = payload.dock_mode {
      current.dock_mode = dock_mode;
      apply_window_layout(&window, &current).map_err(|error| error.to_string())?;
    }
  }

  state.persist().map_err(|error| error.to_string())?;
  Ok(state.window_state.lock().clone())
}

#[tauri::command]
fn toggle_visibility(app: AppHandle) -> Result<(), String> {
  let window = app
    .get_webview_window(WINDOW_LABEL)
    .ok_or_else(|| "main window not found".to_string())?;

  if window.is_visible().map_err(|error| error.to_string())? {
    window.hide().map_err(|error| error.to_string())
  } else {
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
  }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_log::Builder::default().build())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .setup(|app| {
      let app_data_dir = app
        .path()
        .app_data_dir()
        .context("failed to resolve app data directory")?;
      let persistence = PersistenceStore::new(&app_data_dir)?;
      let snapshot = persistence.load().unwrap_or_default();

      let state = AppState::new(snapshot, persistence);
      app.manage(state);

      let window = app
        .get_webview_window(WINDOW_LABEL)
        .ok_or_else(|| anyhow::anyhow!("main window not found"))?;

      configure_window(&window, app.handle())?;
      install_tray(app)?;
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      attach_session,
      bootstrap,
      close_session,
      create_session,
      resize_session,
      send_input,
      set_active_session,
      toggle_visibility,
      update_window_mode,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

fn configure_window(window: &WebviewWindow, app: &AppHandle) -> anyhow::Result<()> {
  let state = app.state::<AppState>();
  let snapshot = state.window_state.lock().clone();

  window.set_always_on_top(snapshot.always_on_top)?;
  window.set_ignore_cursor_events(snapshot.click_through)?;
  apply_window_layout(window, &snapshot)?;

  let app_handle = app.clone();
  window.on_window_event(move |event| match event {
    WindowEvent::CloseRequested { api, .. } => {
      api.prevent_close();
      if let Some(window) = app_handle.get_webview_window(WINDOW_LABEL) {
        let _ = window.hide();
      }
    }
    WindowEvent::Focused(is_focused) => {
      let state = app_handle.state::<AppState>();
      if state.window_state.lock().click_through {
        return;
      }

      let level = if *is_focused {
        OpacityMode::Focus
      } else {
        state.window_state.lock().opacity_mode.clone()
      };
      let _ = app_handle.emit("window-opacity-sync", serialize_opacity(level));
    }
    WindowEvent::Moved(position) => {
      let state = app_handle.state::<AppState>();
      let mut window_state = state.window_state.lock();
      window_state.x = Some(position.x as f64);
      window_state.y = Some(position.y as f64);
      let _ = state.persist();
    }
    WindowEvent::Resized(size) => {
      let state = app_handle.state::<AppState>();
      let mut window_state = state.window_state.lock();
      window_state.width = size.width as f64;
      window_state.height = size.height as f64;
      let _ = state.persist();
    }
    _ => {}
  });

  app.emit("window-opacity-sync", serialize_opacity(snapshot.opacity_mode))?;
  Ok(())
}

fn apply_window_layout(window: &WebviewWindow, window_state: &WindowState) -> anyhow::Result<()> {
  let monitor = window.current_monitor()?;
  let (monitor_width, monitor_height) = monitor
    .map(|monitor| {
      (
        monitor.size().width as f64 / monitor.scale_factor(),
        monitor.size().height as f64 / monitor.scale_factor(),
      )
    })
    .unwrap_or((1920.0_f64, 1080.0_f64));

  let (width, height, x, y) = match window_state.dock_mode {
    DockMode::TopBar => {
      let width = (monitor_width * 0.86).max(1100.0).min(window_state.width.max(1100.0));
      let x = ((monitor_width - width) / 2.0).max(24.0);
      let y = window_state.y.unwrap_or(24.0).max(8.0);
      (width, window_state.height.max(280.0).min(520.0), x, y)
    }
    DockMode::RightRail => {
      let width = window_state.width.max(560.0).min(820.0);
      let height = (monitor_height * 0.86).max(600.0);
      let x = monitor_width - width - 24.0;
      let y = 24.0;
      (width, height, x, y)
    }
  };

  window.set_size(LogicalSize::new(width, height))?;
  window.set_position(LogicalPosition::new(window_state.x.unwrap_or(x), window_state.y.unwrap_or(y)))?;
  Ok(())
}

fn install_tray(app: &mut tauri::App) -> anyhow::Result<()> {
  let app_handle = app.handle().clone();

  TrayIconBuilder::new()
    .tooltip("ClearCodex")
    .show_menu_on_left_click(false)
    .on_tray_icon_event(move |tray, event| {
      if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
      } = event
      {
        if let Some(window) = tray.app_handle().get_webview_window(WINDOW_LABEL) {
          let visible = window.is_visible().unwrap_or(false);
          if visible {
            let _ = window.hide();
          } else {
            let _ = window.show();
            let _ = window.set_focus();
          }
        }
      }
    })
    .build(app)?;

  register_hotkeys(&app_handle);
  Ok(())
}

fn register_hotkeys(app: &AppHandle) {
  use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

  let shortcuts = [
    ("Ctrl+Alt+Space", "overlay.toggle"),
    ("Ctrl+Alt+N", "session.new"),
    ("Ctrl+Alt+T", "window.click_through"),
    ("Ctrl+Alt+Left", "session.previous"),
    ("Ctrl+Alt+Right", "session.next"),
  ];

  for (accelerator, event_name) in shortcuts {
    let Ok(shortcut) = accelerator.parse::<Shortcut>() else {
      continue;
    };
    let app_handle = app.clone();
    let event_name = event_name.to_string();
    let result = app.global_shortcut().on_shortcut(shortcut, move |app, _shortcut, event| {
      if event.state() == ShortcutState::Pressed {
        if event_name == "overlay.toggle" {
          let _ = toggle_visibility(app.clone());
        } else {
          let _ = app.emit("hotkey-event", event_name.clone());
        }
      }
    });

    if let Err(error) = result {
      if let Some(state) = app.try_state::<AppState>() {
        state.push_notice(UiNoticeEvent {
          level: "warning".into(),
          title: "Hotkey registration failed".into(),
          detail: format!("{accelerator}: {error}"),
        });
      }
      let _ = app_handle.emit(
        "ui-notice",
        UiNoticeEvent {
          level: "warning".into(),
          title: "Hotkey registration failed".into(),
          detail: format!("{accelerator}: {error}"),
        },
      );
    }
  }
}

fn serialize_opacity(mode: OpacityMode) -> serde_json::Value {
  let alpha = match mode {
    OpacityMode::Focus => DEFAULT_FOCUS_ALPHA,
    OpacityMode::Peek => DEFAULT_PEEK_ALPHA,
  };

  serde_json::json!({
    "mode": mode,
    "alpha": alpha,
  })
}
