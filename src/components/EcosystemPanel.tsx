import { useState } from 'react';
import {
  BellRing,
  Cpu,
  Gamepad2,
  MicOff,
  Power,
  Radio,
  ShieldAlert,
  Volume2,
  Wifi,
} from 'lucide-react';
import type { AppConfig } from '../types/controller';

interface EcosystemPanelProps {
  config: AppConfig;
  autostartEnabled: boolean;
  onPreferencesChange: (patch: Partial<AppConfig>) => void;
  onToggleAutostart: () => Promise<void>;
  onTestOpenRgb: (addr: string) => Promise<string>;
  onTestWled: (addr: string) => Promise<string>;
}

export function EcosystemPanel({
  config,
  autostartEnabled,
  onPreferencesChange,
  onToggleAutostart,
  onTestOpenRgb,
  onTestWled,
}: EcosystemPanelProps) {
  const [openRgbInput, setOpenRgbInput] = useState(config.openrgb_addr);
  const [wledInput, setWledInput] = useState(config.wled_addr);
  const [bridgeFeedback, setBridgeFeedback] = useState<string | null>(null);
  const [bridgeIsError, setBridgeIsError] = useState(false);

  const runBridgeTest = async (fn: () => Promise<string>) => {
    try {
      const msg = await fn();
      setBridgeIsError(false);
      setBridgeFeedback(msg);
    } catch (err) {
      setBridgeIsError(true);
      setBridgeFeedback(String(err));
    }
  };

  return (
    <section
      aria-label="Ecosistema RGB, Telemetría y Sistema"
      data-testid="ecosystem-panel"
      className="panel-enter flex w-full min-w-0 flex-col gap-2"
    >
      {/* Zero-Injection Game Telemetry Card */}
      <div className="glass-card rounded-2xl p-2.5">
        <div className="flex items-center justify-between gap-2">
          <div className="min-w-0">
            <p className="inline-flex items-center gap-1.5 text-[11px] font-semibold text-slate-100">
              <Gamepad2 className="h-3.5 w-3.5 text-cyan-400" aria-hidden="true" />
              Telemetría Juegos (Zero Memory Injection)
            </p>
            <p className="font-mono-tabular mt-0.5 text-[9.5px] text-slate-300">
              F1 UDP :20777 · Forza :5300 · Valve GSI :3000 · LoL :2999
            </p>
          </div>
          <button
            type="button"
            data-testid="toggle-game-integrations"
            aria-pressed={config.game_integrations_enabled}
            onClick={() =>
              onPreferencesChange({
                game_integrations_enabled: !config.game_integrations_enabled,
              })
            }
            className={`ps-action-tile shrink-0 rounded-xl px-2.5 py-1.5 text-[10px] font-semibold focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 ${
              config.game_integrations_enabled
                ? 'border-emerald-400/50 bg-emerald-500/20 text-emerald-200'
                : 'text-slate-300'
            }`}
          >
            {config.game_integrations_enabled ? 'Activo' : 'Inactivo'}
          </button>
        </div>
      </div>

      {/* External RGB Sync-Out: OpenRGB SDK & WLED UDP */}
      <div className="glass-card flex flex-col gap-2 rounded-2xl p-2.5">
        <div className="flex items-center justify-between">
          <span className="inline-flex items-center gap-1.5 text-[11px] font-semibold text-slate-100">
            <Radio className="h-3.5 w-3.5 text-violet-400" aria-hidden="true" />
            Sync-Out RGB (PC & Tiras LED)
          </span>
          <span className="font-mono-tabular text-[9.5px] text-slate-400">OpenRGB · WLED</span>
        </div>

        {/* OpenRGB Row */}
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            data-testid="toggle-openrgb"
            aria-pressed={config.openrgb_enabled}
            onClick={() =>
              onPreferencesChange({
                openrgb_enabled: !config.openrgb_enabled,
                openrgb_addr: openRgbInput.trim() || config.openrgb_addr,
              })
            }
            className={`ps-action-tile inline-flex w-20 shrink-0 items-center justify-center gap-1 rounded-lg px-2 py-1 text-[10px] font-medium ${
              config.openrgb_enabled
                ? 'border-violet-400/50 bg-violet-500/20 text-violet-100'
                : 'text-slate-300'
            }`}
          >
            <Cpu className="h-3 w-3" aria-hidden="true" />
            OpenRGB
          </button>
          <input
            type="text"
            aria-label="Dirección servidor OpenRGB SDK"
            value={openRgbInput}
            onChange={(e) => setOpenRgbInput(e.target.value)}
            onBlur={() => {
              const trimmed = openRgbInput.trim();
              if (trimmed && trimmed !== config.openrgb_addr) {
                onPreferencesChange({ openrgb_addr: trimmed });
              }
            }}
            className="font-mono-tabular min-w-0 flex-1 rounded-lg border border-white/10 bg-slate-950/35 px-2 py-1 text-[10px] text-slate-100 focus:border-violet-400/50 focus:outline-none"
          />
          <button
            type="button"
            data-testid="test-openrgb-btn"
            onClick={() => void runBridgeTest(() => onTestOpenRgb(openRgbInput))}
            className="ps-action-tile shrink-0 rounded-lg px-2 py-1 text-[10px] font-medium text-slate-200"
          >
            Probar
          </button>
        </div>

        {/* WLED Row */}
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            data-testid="toggle-wled"
            aria-pressed={config.wled_enabled}
            onClick={() =>
              onPreferencesChange({
                wled_enabled: !config.wled_enabled,
                wled_addr: wledInput.trim() || config.wled_addr,
              })
            }
            className={`ps-action-tile inline-flex w-20 shrink-0 items-center justify-center gap-1 rounded-lg px-2 py-1 text-[10px] font-medium ${
              config.wled_enabled
                ? 'border-cyan-400/50 bg-cyan-500/20 text-cyan-100'
                : 'text-slate-300'
            }`}
          >
            <Wifi className="h-3 w-3" aria-hidden="true" />
            WLED UDP
          </button>
          <input
            type="text"
            aria-label="Dirección UDP WLED"
            value={wledInput}
            onChange={(e) => setWledInput(e.target.value)}
            onBlur={() => {
              const trimmed = wledInput.trim();
              if (trimmed && trimmed !== config.wled_addr) {
                onPreferencesChange({ wled_addr: trimmed });
              }
            }}
            className="font-mono-tabular min-w-0 flex-1 rounded-lg border border-white/10 bg-slate-950/35 px-2 py-1 text-[10px] text-slate-100 focus:border-cyan-400/50 focus:outline-none"
          />
          <button
            type="button"
            data-testid="test-wled-btn"
            onClick={() => void runBridgeTest(() => onTestWled(wledInput))}
            className="ps-action-tile shrink-0 rounded-lg px-2 py-1 text-[10px] font-medium text-slate-200"
          >
            Probar
          </button>
        </div>

        {bridgeFeedback && (
          <p
            data-testid="bridge-feedback"
            className={`font-mono-tabular rounded-lg border px-2 py-1 text-[9.5px] ${
              bridgeIsError
                ? 'border-amber-400/30 bg-amber-500/10 text-amber-200'
                : 'border-emerald-400/30 bg-emerald-500/10 text-emerald-200'
            }`}
          >
            {bridgeFeedback}
          </p>
        )}
      </div>

      {/* System Guards & Notifications */}
      <div className="glass-card rounded-2xl p-2.5">
        <div className="mb-1.5 flex items-center justify-between">
          <span className="inline-flex items-center gap-1.5 text-[11px] font-semibold text-slate-100">
            <ShieldAlert className="h-3.5 w-3.5 text-amber-300" aria-hidden="true" />
            Protecciones SO, Alertas & Inicio
          </span>
        </div>

        <div className="grid grid-cols-2 gap-1.5">
          <button
            type="button"
            data-testid="toggle-low-battery-alerts"
            aria-pressed={config.low_battery_alerts}
            onClick={() =>
              onPreferencesChange({ low_battery_alerts: !config.low_battery_alerts })
            }
            className={`ps-action-tile inline-flex items-center justify-center gap-1.5 rounded-xl px-2 py-1.5 text-[10px] font-medium ${
              config.low_battery_alerts
                ? 'border-amber-400/50 text-amber-200'
                : 'text-slate-300'
            }`}
          >
            <BellRing className="h-3 w-3 shrink-0" aria-hidden="true" />
            Alertas 20/10/5%
          </button>

          <button
            type="button"
            data-testid="toggle-autostart"
            aria-pressed={autostartEnabled}
            onClick={() => void onToggleAutostart()}
            className={`ps-action-tile inline-flex items-center justify-center gap-1.5 rounded-xl px-2 py-1.5 text-[10px] font-medium ${
              autostartEnabled ? 'border-emerald-400/50 text-emerald-200' : 'text-slate-300'
            }`}
          >
            <Power className="h-3 w-3 shrink-0" aria-hidden="true" />
            Iniciar con SO
          </button>

          <button
            type="button"
            data-testid="toggle-mic-sync"
            aria-pressed={config.mic_mute_sync_enabled}
            onClick={() =>
              onPreferencesChange({ mic_mute_sync_enabled: !config.mic_mute_sync_enabled })
            }
            className={`ps-action-tile inline-flex items-center justify-center gap-1.5 rounded-xl px-2 py-1.5 text-[10px] font-medium ${
              config.mic_mute_sync_enabled
                ? 'border-sky-400/50 text-sky-100'
                : 'text-slate-300'
            }`}
          >
            <MicOff className="h-3 w-3 shrink-0" aria-hidden="true" />
            Mic Mute Sync
          </button>

          <button
            type="button"
            data-testid="toggle-audio-guard"
            aria-pressed={config.usb_audio_guard_enabled}
            onClick={() =>
              onPreferencesChange({ usb_audio_guard_enabled: !config.usb_audio_guard_enabled })
            }
            className={`ps-action-tile inline-flex items-center justify-center gap-1.5 rounded-xl px-2 py-1.5 text-[10px] font-medium ${
              config.usb_audio_guard_enabled
                ? 'border-violet-400/50 text-violet-100'
                : 'text-slate-300'
            }`}
          >
            <Volume2 className="h-3 w-3 shrink-0" aria-hidden="true" />
            USB Audio Guard
          </button>
        </div>
      </div>
    </section>
  );
}
