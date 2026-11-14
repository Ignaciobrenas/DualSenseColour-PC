import { Bluetooth, Usb, WifiOff } from 'lucide-react';
import type { ConnectionStatus } from '../types/controller';

interface ConnectionBadgeProps {
  status: ConnectionStatus;
}

export function ConnectionBadge({ status }: ConnectionBadgeProps) {
  if (!status.connected) {
    return (
      <span
        data-testid="connection-badge"
        className="inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-white/[0.04] px-2.5 py-1 text-[11px] font-medium text-slate-400"
      >
        <WifiOff className="h-3 w-3 text-slate-500" aria-hidden="true" />
        Sin mando
      </span>
    );
  }

  const isUsb = status.transport === 'usb';
  const modelLabel = status.model === 'DualSenseEdge' ? 'Edge' : 'DualSense';

  return (
    <span
      data-testid="connection-badge"
      className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] font-medium ${
        isUsb
          ? 'border-emerald-400/25 bg-emerald-500/12 text-emerald-300'
          : 'border-sky-400/25 bg-sky-500/12 text-sky-300'
      }`}
    >
      {isUsb ? (
        <Usb className="h-3 w-3 shrink-0" aria-hidden="true" />
      ) : (
        <Bluetooth className="h-3 w-3 shrink-0" aria-hidden="true" />
      )}
      <span>
        {modelLabel} · {isUsb ? 'USB' : 'Bluetooth'}
      </span>
    </span>
  );
}
