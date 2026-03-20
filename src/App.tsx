import { useEffect, useMemo, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import clsx from 'clsx';

import { ControlHandle } from './ControlHandle';
import { TerminalSurface } from './components/TerminalSurface';
import './App.css';
import type {
  AppLanguage,
  BootstrapPayload,
  SessionMetadata,
  SessionStatusEvent,
  UiNotice,
  WindowState,
} from './types';

const EMPTY_WINDOW_STATE: WindowState = {
  alwaysOnTop: true,
  clickThrough: false,
  overlayAlpha: 0.16,
  dockMode: 'top_bar',
  language: 'zh_cn',
  onboardingCompleted: false,
  positionPinned: false,
  width: 1440,
  height: 340,
  x: null,
  y: 24,
};

const copy = {
  zh_cn: {
    eyebrow: '透明悬浮 Codex',
    newTab: '新建会话',
    hide: '隐藏',
    clickThroughOn: '关闭穿透',
    clickThroughOff: '开启穿透',
    pinnedOn: '取消固定',
    pinnedOff: '固定当前位置',
    topBar: '顶部横条',
    rightRail: '右侧窄栏',
    lang: '语言',
    transparency: '透明度',
    workspace: '工作目录',
    hotkeys: '快捷键',
    mode: '窗口模式',
    interactive: '可交互',
    passThrough: '已穿透',
    noSessions: '还没有 Codex 会话',
    startFirst: '开始第一个会话',
    overlayReady: '窗口已经准备好，先新建一个 Codex 会话。',
    attach: '连接',
    end: '结束',
    settings: '设置',
    onboardingTitle: '首次使用引导',
    onboardingBody:
      '1. 拖动顶部栏把窗口放到合适位置。2. 点“固定当前位置”保存位置。3. 用透明度滑杆把它调到刚好不挡视线。4. 点“新建会话”启动 Codex。',
    onboardingSecondary:
      '如果只想看输出，不想遮挡鼠标操作，可以开启穿透。之后按 Ctrl+Alt+Space 随时显示/隐藏。',
    onboardingAction: '开始使用',
    onboardingDismiss: '我知道了',
    sessionIssue: '终端问题',
    failedCreate: '启动 Codex 失败',
    failedPin: '位置固定失败',
    failedState: '窗口设置失败',
    passThroughHint: '已开启穿透，8 秒后会自动恢复。也可以按 Ctrl+Alt+T 立即恢复。',
    failedBootstrap: '初始化失败',
    failedPersist: '保存标签失败',
    failedClose: '关闭会话失败',
    failedAttach: '恢复会话失败',
    live: '运行中',
    starting: '启动中',
    detached: '已分离',
    exited: '已退出',
    failed: '失败',
  },
  en: {
    eyebrow: 'Minimal transparent Codex',
    newTab: 'New session',
    hide: 'Hide',
    clickThroughOn: 'Disable pass-through',
    clickThroughOff: 'Enable pass-through',
    pinnedOn: 'Unpin position',
    pinnedOff: 'Pin current position',
    topBar: 'Top bar',
    rightRail: 'Right rail',
    lang: 'Language',
    transparency: 'Transparency',
    workspace: 'Workspace',
    hotkeys: 'Hotkeys',
    mode: 'Window mode',
    interactive: 'Interactive',
    passThrough: 'Pass-through on',
    noSessions: 'No Codex sessions yet',
    startFirst: 'Start first session',
    overlayReady: 'The overlay is ready. Start a Codex session to begin.',
    attach: 'Attach',
    end: 'End',
    settings: 'Settings',
    onboardingTitle: 'Quick start',
    onboardingBody:
      '1. Drag the top bar to where you want it. 2. Click “Pin current position” to keep it there. 3. Use the transparency slider until it stops blocking your view. 4. Click “New session” to launch Codex.',
    onboardingSecondary:
      'Enable pass-through when you only want to watch output. Use Ctrl+Alt+Space any time to show or hide the overlay.',
    onboardingAction: 'Start using it',
    onboardingDismiss: 'Dismiss',
    sessionIssue: 'Terminal issue',
    failedCreate: 'Failed to start Codex',
    failedPin: 'Failed to pin window position',
    failedState: 'Failed to update window settings',
    passThroughHint: 'Pass-through is active. It will auto-recover in 8 seconds, or press Ctrl+Alt+T now.',
    failedBootstrap: 'Bootstrap failed',
    failedPersist: 'Failed to persist active tab',
    failedClose: 'Failed to close session',
    failedAttach: 'Failed to attach session',
    live: 'Live',
    starting: 'Starting',
    detached: 'Detached',
    exited: 'Exited',
    failed: 'Failed',
  },
} as const;

function getStatusLabel(language: AppLanguage, status: SessionMetadata['status']) {
  const dict = copy[language];
  switch (status) {
    case 'running':
      return dict.live;
    case 'starting':
      return dict.starting;
    case 'detached':
      return dict.detached;
    case 'failed':
      return dict.failed;
    case 'exited':
      return dict.exited;
    default:
      return status;
  }
}

export default function App() {
  if (window.location.hash === '#handle') {
    return <ControlHandle />;
  }

  return <MainApp />;
}

function MainApp() {
  const [ready, setReady] = useState(false);
  const [defaultCwd, setDefaultCwd] = useState('.');
  const [sessions, setSessions] = useState<SessionMetadata[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [windowState, setWindowState] = useState<WindowState>(EMPTY_WINDOW_STATE);
  const [hotkeySummary, setHotkeySummary] = useState<string[]>([]);
  const [notices, setNotices] = useState<UiNotice[]>([]);
  const [busy, setBusy] = useState(false);
  const clickThroughTimerRef = useRef<number | null>(null);

  const activeIndex = useMemo(
    () => sessions.findIndex((session) => session.id === activeSessionId),
    [activeSessionId, sessions],
  );
  const activeSession =
    sessions[activeIndex] ?? sessions.find((session) => session.status !== 'exited') ?? null;
  const dict = copy[windowState.language];

  useEffect(() => {
    document.documentElement.style.setProperty('--overlay-alpha', `${windowState.overlayAlpha}`);
  }, [windowState.overlayAlpha]);

  /* eslint-disable react-hooks/exhaustive-deps */
  useEffect(() => {
    if (clickThroughTimerRef.current) {
      window.clearTimeout(clickThroughTimerRef.current);
      clickThroughTimerRef.current = null;
    }

    if (!windowState.clickThrough) {
      return;
    }

    pushNotice({
      level: 'info',
      title: 'Pass-through',
      detail: dict.passThroughHint,
    });

    clickThroughTimerRef.current = window.setTimeout(() => {
      void applyWindowMode({ clickThrough: false });
    }, 8000);

    return () => {
      if (clickThroughTimerRef.current) {
        window.clearTimeout(clickThroughTimerRef.current);
        clickThroughTimerRef.current = null;
      }
    };
  }, [dict.passThroughHint, windowState.clickThrough]);
  /* eslint-enable react-hooks/exhaustive-deps */

  useEffect(() => {
    void (async () => {
      try {
        const bootstrap = await invoke<BootstrapPayload>('bootstrap');
        setSessions(bootstrap.snapshot.sessions);
        setActiveSessionId(
          bootstrap.snapshot.activeSessionId ??
            bootstrap.snapshot.sessions.find((session) => session.status !== 'exited')?.id ??
            null,
        );
        setWindowState(bootstrap.snapshot.window);
        setNotices(bootstrap.notices);
        setHotkeySummary(bootstrap.hotkeySummary);
        setDefaultCwd(bootstrap.defaultCwd);
      } catch (error) {
        pushNotice({
          level: 'error',
          title: copy.zh_cn.failedBootstrap,
          detail: String(error),
        });
      } finally {
        setReady(true);
      }
    })();
  }, []);

  /* eslint-disable react-hooks/exhaustive-deps */
  useEffect(() => {
    const detachHotkey = listen<string>('hotkey-event', (event) => {
      if (event.payload === 'session.new') {
        void handleCreateSession();
      }

      if (event.payload === 'window.click_through') {
        void applyWindowMode({ clickThrough: !windowState.clickThrough });
      }

      if (event.payload === 'session.previous' && sessions.length > 1) {
        const index = activeIndex <= 0 ? sessions.length - 1 : activeIndex - 1;
        void handleSelectSession(sessions[index]?.id ?? null);
      }

      if (event.payload === 'session.next' && sessions.length > 1) {
        const index = activeIndex >= sessions.length - 1 ? 0 : activeIndex + 1;
        void handleSelectSession(sessions[index]?.id ?? null);
      }
    });

    const detachNotice = listen<UiNotice>('ui-notice', (event) => {
      pushNotice(event.payload);
    });

    const detachStatus = listen<SessionStatusEvent>('session-status', (event) => {
      setSessions((current) =>
        current.map((session) =>
          session.id === event.payload.sessionId
            ? {
                ...session,
                status: event.payload.status,
                exitCode: event.payload.exitCode,
                reconnectable: false,
              }
            : session,
        ),
      );
    });

    return () => {
      void detachHotkey.then((unlisten) => unlisten());
      void detachNotice.then((unlisten) => unlisten());
      void detachStatus.then((unlisten) => unlisten());
    };
  }, [activeIndex, sessions, windowState]);
  /* eslint-enable react-hooks/exhaustive-deps */

  function pushNotice(notice: UiNotice) {
    setNotices((current) => [notice, ...current].slice(0, 6));
  }

  async function persistActiveSession(sessionId: string | null) {
    await invoke('set_active_session', { payload: { sessionId } });
  }

  async function applyWindowMode(payload: Partial<WindowState> & Record<string, unknown>) {
    try {
      const nextState = await invoke<WindowState>('update_window_mode', { payload });
      setWindowState(nextState);
    } catch (error) {
      pushNotice({
        level: 'warning',
        title: dict.failedState,
        detail: String(error),
      });
    }
  }

  async function handleCreateSession() {
    if (busy) {
      return;
    }

    try {
      setBusy(true);
      const session = await invoke<SessionMetadata>('create_session', {
        payload: { cwd: defaultCwd },
      });
      setSessions((current) => [...current, session]);
      setActiveSessionId(session.id);
      await persistActiveSession(session.id);
    } catch (error) {
      pushNotice({
        level: 'error',
        title: dict.failedCreate,
        detail: String(error),
      });
    } finally {
      setBusy(false);
    }
  }

  async function handleSelectSession(sessionId: string | null) {
    setActiveSessionId(sessionId);
    try {
      await persistActiveSession(sessionId);
    } catch (error) {
      pushNotice({
        level: 'warning',
        title: dict.failedPersist,
        detail: String(error),
      });
    }
  }

  async function handleHide() {
    await invoke('toggle_visibility');
  }

  async function handleCloseSession(sessionId: string) {
    try {
      const updated = await invoke<SessionMetadata>('close_session', {
        payload: { sessionId, mode: 'terminate' },
      });
      setSessions((current) =>
        current.map((session) => (session.id === updated.id ? updated : session)),
      );
      if (sessionId === activeSessionId) {
        const fallback =
          sessions.find((session) => session.id !== sessionId && session.status === 'running')?.id ??
          null;
        await handleSelectSession(fallback);
      }
    } catch (error) {
      pushNotice({
        level: 'warning',
        title: dict.failedClose,
        detail: String(error),
      });
    }
  }

  async function handleAttachSession(sessionId: string) {
    try {
      const updated = await invoke<SessionMetadata>('attach_session', { sessionId });
      setSessions((current) =>
        current.map((session) => (session.id === updated.id ? updated : session)),
      );
      await handleSelectSession(updated.id);
    } catch (error) {
      pushNotice({
        level: 'warning',
        title: dict.failedAttach,
        detail: String(error),
      });
    }
  }

  async function handlePinToggle() {
    try {
      const nextState = await invoke<WindowState>('pin_window_position', {
        payload: { pinned: !windowState.positionPinned },
      });
      setWindowState(nextState);
    } catch (error) {
      pushNotice({
        level: 'warning',
        title: dict.failedPin,
        detail: String(error),
      });
    }
  }

  async function completeOnboarding() {
    await applyWindowMode({ onboardingCompleted: true });
  }

  if (!ready) {
    return <main className="shell shell-loading">Booting overlay...</main>;
  }

  return (
    <main
      className={clsx('shell', {
        'shell-right-rail': windowState.dockMode === 'right_rail',
        'shell-click-through': windowState.clickThrough,
      })}
    >
      <header className="chrome" data-tauri-drag-region>
        <div className="title-group">
          <p className="eyebrow">{dict.eyebrow}</p>
          <h1>ClearCodex</h1>
        </div>
        <div className="toolbar">
          <button onClick={() => void handlePinToggle()}>
            {windowState.positionPinned ? dict.pinnedOn : dict.pinnedOff}
          </button>
          <button onClick={() => void applyWindowMode({ clickThrough: !windowState.clickThrough })}>
            {windowState.clickThrough ? dict.clickThroughOn : dict.clickThroughOff}
          </button>
          <button onClick={() => void applyWindowMode({ alwaysOnTop: !windowState.alwaysOnTop })}>
            {windowState.alwaysOnTop ? 'Top' : 'Free'}
          </button>
          <button
            onClick={() =>
              void applyWindowMode({
                dockMode: windowState.dockMode === 'top_bar' ? 'right_rail' : 'top_bar',
              })
            }
          >
            {windowState.dockMode === 'top_bar' ? dict.rightRail : dict.topBar}
          </button>
          <button className="primary" onClick={() => void handleCreateSession()} disabled={busy}>
            {dict.newTab}
          </button>
          <button onClick={() => void handleHide()}>{dict.hide}</button>
        </div>
      </header>

      <section className="settings-bar">
        <label className="setting-chip">
          <span>{dict.transparency}</span>
          <input
            type="range"
            min="4"
            max="48"
            value={Math.round(windowState.overlayAlpha * 100)}
            onChange={(event) =>
              void applyWindowMode({
                overlayAlpha: Number(event.currentTarget.value) / 100,
              })
            }
          />
          <strong>{Math.round((1 - windowState.overlayAlpha) * 100)}%</strong>
        </label>

        <div className="setting-chip">
          <span>{dict.lang}</span>
          <div className="segmented">
            <button
              className={clsx({ active: windowState.language === 'zh_cn' })}
              onClick={() => void applyWindowMode({ language: 'zh_cn' })}
            >
              中文
            </button>
            <button
              className={clsx({ active: windowState.language === 'en' })}
              onClick={() => void applyWindowMode({ language: 'en' })}
            >
              EN
            </button>
          </div>
        </div>
      </section>

      <section className="session-strip">
        {sessions.length ? (
          sessions.map((session) => (
            <article
              key={session.id}
              className={clsx('session-pill', {
                'session-pill-active': session.id === activeSession?.id,
              })}
              onClick={() => void handleSelectSession(session.id)}
            >
              <div className="session-pill-main">
                <strong>{session.title}</strong>
                <span>{getStatusLabel(windowState.language, session.status)}</span>
              </div>
              <div className="session-pill-meta">
                <span>{session.cwd}</span>
                {session.status === 'detached' ? (
                  <button
                    className="inline-button"
                    onClick={(event) => {
                      event.stopPropagation();
                      void handleAttachSession(session.id);
                    }}
                  >
                    {dict.attach}
                  </button>
                ) : null}
                <button
                  className="inline-button"
                  onClick={(event) => {
                    event.stopPropagation();
                    void handleCloseSession(session.id);
                  }}
                >
                  {dict.end}
                </button>
              </div>
            </article>
          ))
        ) : (
          <article className="empty-state">
            <p>{dict.noSessions}</p>
            <button className="primary" onClick={() => void handleCreateSession()}>
              {dict.startFirst}
            </button>
          </article>
        )}
      </section>

      <section className="workspace">
        <aside className="sidebar">
          <div className="sidebar-card">
            <p className="eyebrow">{dict.workspace}</p>
            <strong>{defaultCwd}</strong>
          </div>
          <div className="sidebar-card">
            <p className="eyebrow">{dict.mode}</p>
            <strong>{windowState.dockMode === 'top_bar' ? dict.topBar : dict.rightRail}</strong>
            <span>{windowState.clickThrough ? dict.passThrough : dict.interactive}</span>
          </div>
          <div className="sidebar-card">
            <p className="eyebrow">{dict.hotkeys}</p>
            <ul>
              {hotkeySummary.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          </div>
        </aside>

        <section className="terminal-stack">
          {activeSession ? (
            sessions.map((session) => (
              <TerminalSurface
                key={session.id}
                active={session.id === activeSession.id}
                session={session}
                onError={(detail) =>
                  pushNotice({
                    level: 'warning',
                    title: `${dict.sessionIssue}: ${session.title}`,
                    detail,
                  })
                }
              />
            ))
          ) : (
            <div className="terminal-placeholder">
              <p>{dict.overlayReady}</p>
            </div>
          )}
        </section>
      </section>

      <aside className="notice-stack">
        {notices.map((notice, index) => (
          <article className={clsx('notice', `notice-${notice.level}`)} key={`${notice.title}-${index}`}>
            <strong>{notice.title}</strong>
            <p>{notice.detail}</p>
          </article>
        ))}
      </aside>

      {!windowState.onboardingCompleted ? (
        <section className="onboarding">
          <div className="onboarding-card">
            <p className="eyebrow">{dict.settings}</p>
            <h2>{dict.onboardingTitle}</h2>
            <p>{dict.onboardingBody}</p>
            <p>{dict.onboardingSecondary}</p>
            <div className="onboarding-actions">
              <button className="primary" onClick={() => void completeOnboarding()}>
                {dict.onboardingAction}
              </button>
              <button onClick={() => void completeOnboarding()}>{dict.onboardingDismiss}</button>
            </div>
          </div>
        </section>
      ) : null}
    </main>
  );
}
