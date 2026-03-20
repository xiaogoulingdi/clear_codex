export function ControlHandle() {
  return (
    <main className="handle-shell">
      <div className="handle-orb" data-tauri-drag-region title="Drag to move the overlay">
        <div className="handle-dot" />
      </div>
    </main>
  );
}
