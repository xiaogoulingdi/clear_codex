import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { FitAddon } from '@xterm/addon-fit';
import { Terminal } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';

import type { SessionMetadata, SessionOutputEvent, SessionStatusEvent } from '../types';

interface TerminalSurfaceProps {
  session: SessionMetadata;
  active: boolean;
  onError: (message: string) => void;
}

export function TerminalSurface({
  session,
  active,
  onError,
}: TerminalSurfaceProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);

  const reportError = (message: string) => {
    onError(message);
  };

  async function fitTerminal() {
    if (!active || !terminalRef.current || !fitAddonRef.current || !hostRef.current) {
      return;
    }

    fitAddonRef.current.fit();

    try {
      await invoke('resize_session', {
        payload: {
          sessionId: session.id,
          cols: terminalRef.current.cols,
          rows: terminalRef.current.rows,
        },
      });
    } catch (error) {
      reportError(String(error));
    }
  }

  /* eslint-disable react-hooks/exhaustive-deps */
  useEffect(() => {
    if (!hostRef.current || terminalRef.current) {
      return;
    }

    const terminal = new Terminal({
      allowTransparency: true,
      convertEol: true,
      cursorBlink: true,
      fontFamily: '"JetBrains Mono", "Cascadia Code", Consolas, monospace',
      fontSize: 14,
      lineHeight: 1.15,
      theme: {
        background: '#00000000',
        foreground: '#ebf1ff',
        cursor: '#9bd6ff',
        selectionBackground: '#c2def933',
        black: '#0f131c',
        red: '#f07877',
        green: '#8fd49d',
        yellow: '#f1d778',
        blue: '#8fc8ff',
        magenta: '#be8cff',
        cyan: '#84ded9',
        white: '#d3def0',
        brightBlack: '#71809d',
        brightRed: '#ffb0af',
        brightGreen: '#b9f0c1',
        brightYellow: '#ffe9a4',
        brightBlue: '#bbddff',
        brightMagenta: '#dfbcff',
        brightCyan: '#b6f1ed',
        brightWhite: '#f7fbff',
      },
    });
    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(hostRef.current);
    terminal.focus();
    terminalRef.current = terminal;
    fitAddonRef.current = fitAddon;

    const disposable = terminal.onData((data) => {
      invoke('send_input', {
        payload: {
          sessionId: session.id,
          data,
        },
      }).catch((error) => {
        reportError(String(error));
      });
    });

    return () => {
      disposable.dispose();
      terminal.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
    };
  }, [session.id]);

  useEffect(() => {
    if (!terminalRef.current) {
      return;
    }

    const detachOutput = listen<SessionOutputEvent>('session-output', (event) => {
      if (event.payload.sessionId === session.id) {
        terminalRef.current?.write(event.payload.data);
      }
    });

    const detachStatus = listen<SessionStatusEvent>('session-status', (event) => {
      if (event.payload.sessionId === session.id) {
        const exitCode = event.payload.exitCode ?? 'unknown';
        terminalRef.current?.writeln(
          `\r\n[clear-codex] session ${event.payload.status} (exit code: ${exitCode})`,
        );
      }
    });

    return () => {
      void detachOutput.then((unlisten) => unlisten());
      void detachStatus.then((unlisten) => unlisten());
    };
  }, [session.id]);

  /* eslint-disable react-hooks/exhaustive-deps */
  useEffect(() => {
    if (!active) {
      return;
    }

    const observer = new ResizeObserver(() => {
      void fitTerminal();
    });

    if (hostRef.current) {
      observer.observe(hostRef.current);
    }

    void fitTerminal();
    terminalRef.current?.focus();

    return () => {
      observer.disconnect();
    };
  }, [active, session.id]);
  /* eslint-enable react-hooks/exhaustive-deps */

  return (
    <section
      className={`terminal-surface${active ? ' terminal-surface-active' : ''}`}
      aria-hidden={!active}
    >
      {!session.reconnectable && session.status === 'detached' ? (
        <div className="terminal-banner">
          Session is detached. Keep the app alive to preserve the live PTY.
        </div>
      ) : null}
      <div className="terminal-host" ref={hostRef} />
    </section>
  );
}
