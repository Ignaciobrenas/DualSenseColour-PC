import { Minus, X } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import iconUrl from '../assets/icon.svg';
import type { ConnectionStatus } from '../types/controller';
import { ConnectionBadge } from './ConnectionBadge';

interface TitleBarProps {
  status: ConnectionStatus;
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export function TitleBar({ status }: TitleBarProps) {
  const handleMinimize = async () => {
    if (!isTauriRuntime()) {
      return;
    }
    try {
      await getCurrentWindow().minimize();
    } catch {
      // Ignore outside native Tauri shell
    }
  };

  const handleClose = async () => {
    if (!isTauriRuntime()) {
      return;
    }
    try {
      await getCurrentWindow().close();
    } catch {
      // Ignore outside native Tauri shell
    }
  };

  return (
    <header
      data-tauri-drag-region
      className="flex w-full shrink-0 items-center justify-between gap-2 border-b border-white/[0.08] px-3 py-2.5"
    >
      <div className="pointer-events-none flex min-w-0 items-center gap-2">
        <img src={iconUrl} alt="DualSenseColour PC" className="h-5 w-5 shrink-0 rounded-md" />
        <h1 className="truncate text-[13px] font-semibold tracking-tight text-slate-100">
          DualSenseColour PC
        </h1>
      </div>

      <div data-tauri-drag-region="false" className="flex shrink-0 items-center gap-1.5">
        <ConnectionBadge status={status} />

        <div className="flex items-center gap-0.5 pl-0.5">
          <button
            type="button"
            aria-label="Minimizar ventana"
            onClick={handleMinimize}
            className="flex h-6 w-6 items-center justify-center rounded-full text-slate-400 transition-colors hover:bg-white/10 hover:text-slate-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-400"
          >
            <Minus className="h-3.5 w-3.5" aria-hidden="true" />
          </button>
          <button
            type="button"
            aria-label="Ocultar en bandeja del sistema"
            onClick={handleClose}
            className="flex h-6 w-6 items-center justify-center rounded-full text-slate-400 transition-colors hover:bg-rose-500/20 hover:text-rose-300 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-400"
          >
            <X className="h-3.5 w-3.5" aria-hidden="true" />
          </button>
        </div>
      </div>
    </header>
  );
}
