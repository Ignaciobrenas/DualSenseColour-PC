import { Bluetooth, Layers, Usb } from 'lucide-react';
import { rgbToHex, type AppConfig, type ConnectionStatus } from '../types/controller';

interface ControllerSwitcherProps {
  status: ConnectionStatus;
  config: AppConfig;
  onSelect: (controllerId: string | null, syncAll?: boolean) => void;
}

/**
 * Compact multi-controller selector bar allowing either broadcast sync across all attached
 * USB/Bluetooth DualSense controllers or independent per-controller configuration.
 */
export function ControllerSwitcher({ status, config, onSelect }: ControllerSwitcherProps) {
  const controllers = status.controllers ?? [];
  if (!status.connected || controllers.length <= 1) {
    return null;
  }

  const syncAll = config.sync_all_controllers;
  const activeId = status.active_controller_id ?? controllers[0]?.id ?? null;

  return (
    <div
      data-testid="controller-switcher"
      className="mb-1.5 flex items-center gap-1 overflow-x-auto rounded-xl border border-white/[0.08] bg-slate-950/35 p-1 backdrop-blur-md"
    >
      <button
        type="button"
        data-testid="controller-sync-all-btn"
        aria-pressed={syncAll}
        onClick={() => onSelect(activeId, true)}
        className={`inline-flex shrink-0 items-center gap-1 rounded-lg px-2 py-1 text-[10px] font-semibold transition-all ${
          syncAll
            ? 'bg-cyan-500/20 text-cyan-200 shadow-[0_0_10px_rgba(6,182,212,0.2)] ring-1 ring-cyan-400/40'
            : 'text-slate-300 hover:bg-white/[0.06] hover:text-white'
        }`}
      >
        <Layers className="h-3 w-3 text-cyan-400" aria-hidden="true" />
        Todos ({controllers.length})
      </button>

      {controllers.map((ctrl) => {
        const isSelected = !syncAll && activeId === ctrl.id;
        const swatchRgb = syncAll
          ? config.color
          : (config.per_controller?.[ctrl.id]?.color ?? config.color);
        const swatchHex = rgbToHex(swatchRgb);
        const batteryText =
          typeof ctrl.battery_percent === 'number' ? `${ctrl.battery_percent}%` : '...';

        return (
          <button
            key={ctrl.id}
            type="button"
            data-testid={`controller-slot-${ctrl.slot}`}
            aria-pressed={isSelected}
            onClick={() => onSelect(ctrl.id, false)}
            className={`inline-flex shrink-0 items-center gap-1.5 rounded-lg px-2 py-1 text-[10px] font-medium transition-all ${
              isSelected
                ? 'bg-white/15 text-white ring-1 ring-white/30'
                : 'text-slate-300 hover:bg-white/[0.06] hover:text-white'
            }`}
          >
            <span
              className="h-2 w-2 rounded-full ring-1 ring-white/40"
              style={{ backgroundColor: swatchHex }}
              aria-hidden="true"
            />
            <span>#{ctrl.slot}</span>
            {ctrl.transport === 'usb' ? (
              <Usb className="h-2.5 w-2.5 text-emerald-400" aria-hidden="true" />
            ) : (
              <Bluetooth className="h-2.5 w-2.5 text-sky-400" aria-hidden="true" />
            )}
            <span className="font-mono-tabular text-[9.5px] text-slate-300">{batteryText}</span>
          </button>
        );
      })}
    </div>
  );
}
