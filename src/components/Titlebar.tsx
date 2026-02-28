import { getCurrentWindow } from '@tauri-apps/api/window';
import { Minus, Square, X } from 'lucide-react';
import appIcon from '../assets/icon.jpg';

const isTauriRuntime = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

const Titlebar = () => {
  const appWindow = isTauriRuntime() ? getCurrentWindow() : null;

  const handleMinimize = () => {
    if (!appWindow) return;
    void appWindow.minimize();
  };

  const handleMaximize = () => {
    if (!appWindow) return;
    void appWindow.toggleMaximize();
  };

  const handleClose = () => {
    if (!appWindow) return;
    void appWindow.close();
  };

  const startDrag = (event: React.MouseEvent) => {
    if (!appWindow) return;

    const target = event.target as HTMLElement;
    if (target.closest('button')) return;

    void appWindow.startDragging();
  };

  return (
    <header className="titlebar" onMouseDown={startDrag}>
      <div className="titlebar-brand">
        <img src={appIcon} alt="Flux Flow" className="titlebar-icon" />
        <span>Flux Flow</span>
      </div>

      <div className="titlebar-controls">
        <button type="button" className="titlebar-btn" onClick={handleMinimize} aria-label="Minimize">
          <Minus size={14} />
        </button>
        <button type="button" className="titlebar-btn" onClick={handleMaximize} aria-label="Maximize">
          <Square size={10} />
        </button>
        <button type="button" className="titlebar-btn titlebar-btn-close" onClick={handleClose} aria-label="Close">
          <X size={14} />
        </button>
      </div>
    </header>
  );
};

export default Titlebar;
