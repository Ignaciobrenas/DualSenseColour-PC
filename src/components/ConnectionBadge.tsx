import { BatteryCharging, Bluetooth, Usb, WifiOff } from 'lucide-react';
import type { ConnectionStatus } from '../types/controller';

interface ConnectionBadgeProps {
  status: ConnectionStatus;
  onResync?: () => void;
}

function batteryColorClass(percent: number): string {
  if (percent >= 50) return 'text-emerald-300';
  if (percent >= 20) return 'text-amber-300';
  return 'text-rose-400';
}

export function ConnectionBadge({ status, onResync }: ConnectionBadgeProps) {
  if (!status.connected) {
    return (
      <button
        type="button"
        onClick={onResync}
        title="Clic para resincronizar mando"
        data-testid="connection-badge"
        className="inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-white/[0.04] px-2 py-1 text-[11px] font-medium text-slate-400 transition-colors hover:border-white/25 hover:text-slate-200"
      >
        <WifiOff className="h-3 w-3 text-slate-500" aria-hidden="true" />
        Sin mando
      </button>
    );
  }

  const isUsb = status.transport === 'usb';
  const modelLabel = status.model === 'DualSenseEdge' ? 'Edge' : 'DualSense';
  const batteryPercent =
    typeof status.battery_percent === 'number' ? status.battery_percent : null;
  const isCharging = status.charging_state === 'charging';
  const multiCount =
    typeof status.connected_count === 'number' && status.connected_count > 1
      ? status.connected_count
      : null;

  return (
    <button
      type="button"
      onClick={onResync}
      title="Clic para resincronizar mando"
      data-testid="connection-badge"
      className={`inline-flex items-center gap-1.5 rounded-full border px-2 py-1 text-[11px] font-medium transition-colors hover:brightness-110 ${
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
      {batteryPercent !== null && (
        <span
          data-testid="battery-badge"
          className={`font-mono-tabular inline-flex items-center gap-0.5 ${batteryColorClass(batteryPercent)}`}
        >
          {isCharging && <BatteryCharging className="h-3 w-3" aria-hidden="true" />}
          · {batteryPercent}%
        </span>
      )}
      {multiCount !== null && (
        <span className="font-mono-tabular rounded bg-white/15 px-1 text-[9px] text-white">
          ×{multiCount}
        </span>
      )}
    </button>
  );
}
