export type SessionStatus = 'starting' | 'running' | 'exited' | 'failed' | 'detached';
export type CloseMode = 'detach' | 'terminate';
export type DockMode = 'top_bar' | 'right_rail';
export type AppLanguage = 'zh_cn' | 'en';

export interface SessionMetadata {
  id: string;
  title: string;
  cwd: string;
  shell: string;
  codexCommand: string;
  status: SessionStatus;
  persistOnClose: boolean;
  pid: number | null;
  exitCode: number | null;
  reconnectable: boolean;
}

export interface WindowState {
  alwaysOnTop: boolean;
  clickThrough: boolean;
  overlayAlpha: number;
  dockMode: DockMode;
  language: AppLanguage;
  onboardingCompleted: boolean;
  positionPinned: boolean;
  width: number;
  height: number;
  x: number | null;
  y: number | null;
}

export interface AppSnapshot {
  window: WindowState;
  sessions: SessionMetadata[];
  activeSessionId: string | null;
}

export interface UiNotice {
  level: 'warning' | 'error' | 'info';
  title: string;
  detail: string;
}

export interface BootstrapPayload {
  snapshot: AppSnapshot;
  notices: UiNotice[];
  hotkeySummary: string[];
  defaultCwd: string;
}

export interface SessionOutputEvent {
  sessionId: string;
  data: string;
}

export interface SessionStatusEvent {
  sessionId: string;
  status: SessionStatus;
  exitCode: number | null;
  message: string | null;
}

export interface WindowOpacitySyncEvent {
  alpha: number;
}
