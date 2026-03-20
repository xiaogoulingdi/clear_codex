import { useEffect, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import clsx from 'clsx';

import { TerminalSurface } from './components/TerminalSurface';
import './App.css';
import type {
  BootstrapPayload,
  SessionMetadata,
  SessionStatusEvent,
  UiNotice,
  WindowOpacitySyncEvent,
  WindowState,
} from './types';

const EMPTY_WINDOW_STATE: WindowState = {
  alwaysOnTop: true,
  clickThrough: false,
  opacityMode: 'focus',
  dockMode: 'top_bar',
  width: 1440,
  height: 340,
  x: null,
  y: 24,
};

function statusLabel(status: SessionMetadata['status']) {
  switch (status) {
    case 'running':
      return 'Live';
    case 'starting':
      return 'Starting';
    case 'detached':
      return 'Detached';
    case 'failed':
      return 'Failed';
    case 'exited':
      return 'Exited';
    default:
      return status;
  }
}

export default function App() {
  const [ready, setReady] = useState(false);
  const [defaultCwd, setDefaultCwd] = useState('.');
  const [sessions, setSessions] = useState<SessionMetadata[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [windowState, setWindowState] = useState<WindowState>(EMPTY_WINDOW_STATE);
  const [hotkeySummary, setHotkeySummary] = useState<string[]>([]);
  const [notices, setNotices] = useState<UiNotice[]>([]);
  const [busy, setBusy] = useState(false);

  const activeIndex = useMemo(
    () => sessions.findIndex((session) => session.id === activeSessionId),
    [activeSessionId, sessions],
  );
  const activeSession =
    sessions[activeIndex] ?? sessions.find((session) => session.status !== 'exited') ?? null;

  const pushNotice = (notice: UiNotice) => {
    setNotices((current) => [notice, ...current].slice(0, 6));
  };

  const persistActiveSession = async (sessionId: string | null) => {
    await invoke('set_active_session', {
      payload: { sessionId },
    });
  };

  const hydrate = async () => {
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
    setReady(true);
  };

  useEffect(() => {
    void hydrate().catch((error) => {
      pushNotice({
        level: 'error',
        title: 'Bootstrap failed',
        detail: String(error),
      });
      setReady(true);
    });
  }, []);

  /* eslint-disable react-hooks/exhaustive-deps */
  useEffect(() => {
    const detachHotkey = listen<string>('hotkey-event', (event) => {
      if (event.payload === 'session.new') {
        void handleCreateSession();
      }

      if (event.payload === 'window.click_through') {
        void toggleClickThrough();
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

    const detachOpacity = listen<WindowOpacitySyncEvent>('window-opacity-sync', (event) => {
      document.documentElement.style.setProperty('--overlay-alpha', `${event.payload.alpha}`);
    });

    return () => {
      void detachHotkey.then((unlisten) => unlisten());
      void detachNotice.then((unlisten) => unlisten());
      void detachStatus.then((unlisten) => unlisten());
      void detachOpacity.then((unlisten) => unlisten());
    };
  }, [activeIndex, pushNotice, sessions]);
  /* eslint-enable react-hooks/exhaustive-deps */

  async function handleCreateSession() {
    if (busy) {
      return;
    }

    try {
      setBusy(true);
      const session = await invoke<SessionMetadata>('create_session', {
        payload: {
          cwd: defaultCwd,
        },
      });
      setSessions((current) => [...current, session]);
      setActiveSessionId(session.id);
      await persistActiveSession(session.id);
    } catch (error) {
      pushNotice({
        level: 'error',
        title: 'Failed to start Codex',
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
        title: 'Failed to persist active tab',
        detail: String(error),
      });
    }
  }

  async function toggleClickThrough() {
    try {
      const nextState = await invoke<WindowState>('update_window_mode', {
        payload: {
          clickThrough: !windowState.clickThrough,
        },
      });
      setWindowState(nextState);
    } catch (error) {
      pushNotice({
        level: 'warning',
        title: 'Click-through toggle failed',
        detail: String(error),
      });
    }
  }

  async function toggleOpacityMode() {
    try {
      const nextState = await invoke<WindowState>('update_window_mode', {
        payload: {
          opacityMode: windowState.opacityMode === 'focus' ? 'peek' : 'focus',
        },
      });
      setWindowState(nextState);
    } catch (error) {
      pushNotice({
        level: 'warning',
        title: 'Opacity update failed',
        detail: String(error),
      });
    }
  }

  async function toggleAlwaysOnTop() {
    try {
      const nextState = await invoke<WindowState>('update_window_mode', {
        payload: {
          alwaysOnTop: !windowState.alwaysOnTop,
        },
      });
      setWindowState(nextState);
    } catch (error) {
      pushNotice({
        level: 'warning',
        title: 'Pin update failed',
        detail: String(error),
      });
    }
  }

  async function toggleDockMode() {
    try {
      const nextState = await invoke<WindowState>('update_window_mode', {
        payload: {
          dockMode: windowState.dockMode === 'top_bar' ? 'right_rail' : 'top_bar',
        },
      });
      setWindowState(nextState);
    } catch (error) {
      pushNotice({
        level: 'warning',
        title: 'Dock mode update failed',
        detail: String(error),
      });
    }
  }

  async function handleCloseSession(sessionId: string) {
    try {
      const updated = await invoke<SessionMetadata>('close_session', {
        payload: {
          sessionId,
          mode: 'terminate',
        },
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
        title: 'Close session failed',
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
        title: 'Attach failed',
        detail: String(error),
      });
    }
  }

  async function handleHide() {
    await invoke('toggle_visibility');
  }

  if (!ready) {
    return <main className="shell loading-shell">Booting overlay...</main>;
  }

  return (
    <main
      className={clsx('shell', {
        'shell-peek': windowState.opacityMode === 'peek',
        'shell-click-through': windowState.clickThrough,
        'shell-right-rail': windowState.dockMode === 'right_rail',
      })}
    >
      <div className="backdrop" />
      <header className="chrome" data-tauri-drag-region>
        <div className="brand">
          <div className="brand-mark">CC</div>
          <div>
            <p className="eyebrow">Codex overlay manager</p>
            <h1>ClearCodex</h1>
          </div>
        </div>
        <div className="controls">
          <button onClick={() => void toggleOpacityMode()}>
            {windowState.opacityMode === 'focus' ? 'Peek opacity' : 'Focus opacity'}
          </button>
          <button onClick={() => void toggleClickThrough()}>
            {windowState.clickThrough ? 'Disable pass-through' : 'Enable pass-through'}
          </button>
          <button onClick={() => void toggleAlwaysOnTop()}>
            {windowState.alwaysOnTop ? 'Pinned' : 'Unpinned'}
          </button>
          <button onClick={() => void toggleDockMode()}>
            {windowState.dockMode === 'top_bar' ? 'Right rail' : 'Top bar'}
          </button>
          <button className="accent" onClick={() => void handleCreateSession()} disabled={busy}>
            New tab
          </button>
          <button onClick={() => void handleHide()}>Hide</button>
        </div>
      </header>

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
                <span>{statusLabel(session.status)}</span>
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
                    Attach
                  </button>
                ) : null}
                <button
                  className="inline-button"
                  onClick={(event) => {
                    event.stopPropagation();
                    void handleCloseSession(session.id);
                  }}
                >
                  End
                </button>
              </div>
            </article>
          ))
        ) : (
          <article className="empty-state">
            <p>No Codex sessions yet.</p>
            <button className="accent" onClick={() => void handleCreateSession()}>
              Start first session
            </button>
          </article>
        )}
      </section>

      <section className="workspace">
        <aside className="sidebar">
          <div>
            <p className="eyebrow">Workspace</p>
            <strong>{defaultCwd}</strong>
          </div>
          <div>
            <p className="eyebrow">Hotkeys</p>
            <ul>
              {hotkeySummary.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          </div>
          <div>
            <p className="eyebrow">Mode</p>
            <strong>{windowState.dockMode === 'top_bar' ? 'Top bar' : 'Right rail'}</strong>
            <span>{windowState.clickThrough ? 'Pointer pass-through on' : 'Interactive'}</span>
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
                    title: `Terminal issue in ${session.title}`,
                    detail,
                  })
                }
              />
            ))
          ) : (
            <div className="terminal-placeholder">
              <p>The overlay is ready. Launch a Codex tab to begin.</p>
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
    </main>
  );
}
