use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
  Starting,
  Running,
  Exited,
  Failed,
  Detached,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloseMode {
  Detach,
  Terminate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DockMode {
  TopBar,
  RightRail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppLanguage {
  ZhCn,
  En,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMetadata {
  pub id: String,
  pub title: String,
  pub cwd: String,
  pub shell: String,
  pub codex_command: String,
  pub status: SessionStatus,
  pub persist_on_close: bool,
  pub pid: Option<u32>,
  pub exit_code: Option<i32>,
  pub reconnectable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
  pub always_on_top: bool,
  pub click_through: bool,
  pub overlay_alpha: f64,
  pub dock_mode: DockMode,
  pub language: AppLanguage,
  pub onboarding_completed: bool,
  pub position_pinned: bool,
  pub width: f64,
  pub height: f64,
  pub x: Option<f64>,
  pub y: Option<f64>,
}

impl Default for WindowState {
  fn default() -> Self {
    Self {
      always_on_top: true,
      click_through: false,
      overlay_alpha: 0.16,
      dock_mode: DockMode::TopBar,
      language: AppLanguage::ZhCn,
      onboarding_completed: false,
      position_pinned: false,
      width: 1440.0,
      height: 340.0,
      x: None,
      y: Some(24.0),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
  pub window: WindowState,
  pub sessions: Vec<SessionMetadata>,
  pub active_session_id: Option<String>,
}

impl Default for AppSnapshot {
  fn default() -> Self {
    Self {
      window: WindowState::default(),
      sessions: Vec::new(),
      active_session_id: None,
    }
  }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionOutputEvent {
  pub session_id: String,
  pub data: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusEvent {
  pub session_id: String,
  pub status: SessionStatus,
  pub exit_code: Option<i32>,
  pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiNoticeEvent {
  pub level: String,
  pub title: String,
  pub detail: String,
}
