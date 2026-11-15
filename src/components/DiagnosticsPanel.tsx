import { useState } from 'react';
import {
  Activity,
  BatteryCharging,
  Check,
  Compass,
  Cpu,
  Download,
  Gamepad2,
  Server,
  Upload,
  Zap,
} from 'lucide-react';
import type { AppConfig, ConnectionStatus, TriggerPreset } from '../types/controller';

interface DiagnosticsPanelProps {
  status: ConnectionStatus;
  config: AppConfig;
  onPreferencesChange: (patch: Partial<AppConfig>) => void;
  onTriggersChange: (left: TriggerPreset, right: TriggerPreset) => void;
  onExportProfile: () => Promise<string>;
  onImportProfile: (json: string) => Promise<void>;
}

const TRIGGER_OPTIONS: { id: TriggerPreset; label: string }[] = [
  { id: 'off', label: 'Libre' },
  { id: 'rigid', label: 'Rígido' },
  { id: 'weapon_click', label: 'Gatillo FPS' },
  { id: 'pulse', label: 'Pulso' },
  { id: 'hair_trigger', label: 'Hair-Trig' },
  { id: 'dual_stage', label: '2-Etapas' },
  { id: 'progressive_pedal', label: 'Pedal Pro' },
];

export function DiagnosticsPanel({
  status,
  config,
  onPreferencesChange,
  onTriggersChange,
  onExportProfile,
  onImportProfile,
}: DiagnosticsPanelProps) {
  const [profileJson, setProfileJson] = useState('');
  const [copiedBanner, setCopiedBanner] = useState(false);

  const telem = status.telemetry;
  const lx = telem?.lx ?? 0;
  const ly = telem?.ly ?? 0;
  const rx = telem?.rx ?? 0;
  const ry = telem?.ry ?? 0;
  const l2Pct = Math.round(((telem?.l2 ?? 0) / 255) * 100);
  const r2Pct = Math.round(((telem?.r2 ?? 0) / 255) * 100);

  const leftDrift = Math.min(100, Math.hypot(lx, ly) * 100);
  const rightDrift = Math.min(100, Math.hypot(rx, ry) * 100);

  const handleExport = async () => {
    const json = await onExportProfile();
    setProfileJson(json);
    try {
      await navigator.clipboard?.writeText(json);
      setCopiedBanner(true);
      window.setTimeout(() => setCopiedBanner(false), 1800);
    } catch {
      // Clipboard optional in sandboxed webviews; textarea still shows JSON
    }
  };

  return (
    <section
      aria-label="Diagnóstico y Suite Pro"
      data-testid="diagnostics-panel"
      className="flex w-full min-w-0 flex-col gap-2"
    >
      {/* Live Stick Drift & Circularity Monitor */}
      <div className="glass-card rounded-2xl p-2.5">
        <div className="mb-2 flex items-center justify-between">
          <span className="inline-flex items-center gap-1.5 text-[11px] font-semibold text-slate-200">
            <Activity className="h-3.5 w-3.5 text-cyan-400" aria-hidden="true" />
            Telemetría · Stick Drift & Gatillos
          </span>
          <span className="font-mono-tabular text-[10px] text-slate-400">
            {telem ? 'HID LIVE' : 'EN ESPERA HID'}
          </span>
        </div>

        <div className="grid grid-cols-2 gap-2">
          <div className="rounded-xl border border-white/[0.06] bg-slate-950/25 p-2">
            <div className="flex items-center justify-between text-[10px] text-slate-400">
              <span>Stick Izq (L3)</span>
              <span
                className={`font-mono-tabular font-semibold ${
                  leftDrift > 6 ? 'text-amber-300' : 'text-emerald-300'
                }`}
              >
                Drift {leftDrift.toFixed(1)}%
              </span>
            </div>
            <p className="font-mono-tabular mt-1 text-[10px] text-slate-300">
              X:{lx.toFixed(2)} · Y:{ly.toFixed(2)} · L2:{l2Pct}%
            </p>
            <div className="mt-1 h-1 w-full overflow-hidden rounded-full bg-white/10">
              <div
                className="h-full rounded-full bg-violet-400 transition-[width] duration-75"
                style={{ width: `${l2Pct}%` }}
              />
            </div>
          </div>

          <div className="rounded-xl border border-white/[0.06] bg-slate-950/25 p-2">
            <div className="flex items-center justify-between text-[10px] text-slate-400">
              <span>Stick Der (R3)</span>
              <span
                className={`font-mono-tabular font-semibold ${
                  rightDrift > 6 ? 'text-amber-300' : 'text-emerald-300'
                }`}
              >
                Drift {rightDrift.toFixed(1)}%
              </span>
            </div>
            <p className="font-mono-tabular mt-1 text-[10px] text-slate-300">
              X:{rx.toFixed(2)} · Y:{ry.toFixed(2)} · R2:{r2Pct}%
            </p>
            <div className="mt-1 h-1 w-full overflow-hidden rounded-full bg-white/10">
              <div
                className="h-full rounded-full bg-cyan-400 transition-[width] duration-75"
                style={{ width: `${r2Pct}%` }}
              />
            </div>
          </div>
        </div>
      </div>

      {/* Adaptive Triggers Probador */}
      <div className="glass-card rounded-2xl p-2.5">
        <div className="mb-1.5 flex items-center justify-between">
          <span className="inline-flex items-center gap-1.5 text-[11px] font-semibold text-slate-200">
            <Zap className="h-3.5 w-3.5 text-violet-400" aria-hidden="true" />
            Probador Gatillos Adaptativos (7 Modos)
          </span>
        </div>

        <div className="flex flex-col gap-1.5">
          <div className="flex items-start gap-1.5">
            <span className="font-mono-tabular w-6 pt-1 text-[10px] font-semibold text-slate-400">
              L2
            </span>
            <div className="grid flex-1 grid-cols-4 gap-1">
              {TRIGGER_OPTIONS.map((opt) => (
                <button
                  key={`left-${opt.id}`}
                  type="button"
                  data-testid={`trigger-left-${opt.id}`}
                  aria-pressed={config.left_trigger === opt.id}
                  onClick={() => onTriggersChange(opt.id, config.right_trigger)}
                  className={`ps-action-tile rounded-lg px-1 py-1 text-[9.5px] font-medium ${
                    config.left_trigger === opt.id
                      ? 'border-violet-400/50 bg-violet-500/20 text-violet-100'
                      : 'text-slate-300'
                  }`}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </div>

          <div className="flex items-start gap-1.5">
            <span className="font-mono-tabular w-6 pt-1 text-[10px] font-semibold text-slate-400">
              R2
            </span>
            <div className="grid flex-1 grid-cols-4 gap-1">
              {TRIGGER_OPTIONS.map((opt) => (
                <button
                  key={`right-${opt.id}`}
                  type="button"
                  data-testid={`trigger-right-${opt.id}`}
                  aria-pressed={config.right_trigger === opt.id}
                  onClick={() => onTriggersChange(config.left_trigger, opt.id)}
                  className={`ps-action-tile rounded-lg px-1 py-1 text-[9.5px] font-medium ${
                    config.right_trigger === opt.id
                      ? 'border-cyan-400/50 bg-cyan-500/20 text-cyan-100'
                      : 'text-slate-300'
                  }`}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Pro Automation Toggles */}
      <div className="grid grid-cols-2 gap-1.5">
        <button
          type="button"
          data-testid="toggle-battery-bar"
          aria-pressed={config.battery_bar_leds}
          onClick={() => onPreferencesChange({ battery_bar_leds: !config.battery_bar_leds })}
          className={`ps-action-tile inline-flex items-center justify-center gap-1.5 rounded-xl px-2 py-1.5 text-[10px] font-medium ${
            config.battery_bar_leds
              ? 'border-emerald-400/45 text-emerald-200'
              : 'text-slate-300'
          }`}
        >
          <BatteryCharging className="h-3 w-3" aria-hidden="true" />
          Batería 5 LED
        </button>

        <button
          type="button"
          data-testid="toggle-multi-sync"
          aria-pressed={config.sync_all_controllers}
          onClick={() =>
            onPreferencesChange({ sync_all_controllers: !config.sync_all_controllers })
          }
          className={`ps-action-tile inline-flex items-center justify-center gap-1.5 rounded-xl px-2 py-1.5 text-[10px] font-medium ${
            config.sync_all_controllers
              ? 'border-sky-400/45 text-sky-100'
              : 'text-slate-300'
          }`}
        >
          <Gamepad2 className="h-3 w-3" aria-hidden="true" />
          Multi-Mando
        </button>

        <button
          type="button"
          data-testid="toggle-local-api"
          aria-pressed={config.local_api_enabled}
          onClick={() => onPreferencesChange({ local_api_enabled: !config.local_api_enabled })}
          className={`ps-action-tile inline-flex items-center justify-center gap-1.5 rounded-xl px-2 py-1.5 text-[10px] font-medium ${
            config.local_api_enabled
              ? 'border-cyan-400/45 text-cyan-100'
              : 'text-slate-300'
          }`}
        >
          <Server className="h-3 w-3" aria-hidden="true" />
          API :17420
        </button>

        <button
          type="button"
          data-testid="toggle-gyro-indicator"
          aria-pressed={config.gyro_indicator_enabled}
          onClick={() =>
            onPreferencesChange({ gyro_indicator_enabled: !config.gyro_indicator_enabled })
          }
          className={`ps-action-tile inline-flex items-center justify-center gap-1.5 rounded-xl px-2 py-1.5 text-[10px] font-medium ${
            config.gyro_indicator_enabled
              ? 'border-emerald-400/50 text-emerald-200'
              : 'text-slate-300'
          }`}
        >
          <Compass className="h-3 w-3" aria-hidden="true" />
          Gyro-Aim Verde
        </button>
      </div>

      {/* Game Auto-Switcher & Shareable .sensehue Profile */}
      <div className="glass-card rounded-2xl p-2.5">
        <div className="mb-1.5 flex items-center justify-between">
          <span className="inline-flex items-center gap-1.5 text-[11px] font-semibold text-slate-200">
            <Cpu className="h-3.5 w-3.5 text-sky-400" aria-hidden="true" />
            Game Auto-Switcher & Perfiles .sensehue
          </span>
          <div className="flex items-center gap-1">
            <button
              type="button"
              data-testid="export-profile-btn"
              onClick={() => void handleExport()}
              className="ps-action-tile inline-flex items-center gap-1 rounded-lg px-2 py-0.5 text-[10px] text-slate-200"
            >
              {copiedBanner ? (
                <Check className="h-3 w-3 text-emerald-300" aria-hidden="true" />
              ) : (
                <Download className="h-3 w-3" aria-hidden="true" />
              )}
              {copiedBanner ? 'Copiado' : 'Exportar'}
            </button>
            <button
              type="button"
              data-testid="import-profile-btn"
              onClick={() => {
                if (profileJson.trim()) {
                  void onImportProfile(profileJson);
                }
              }}
              className="ps-action-tile inline-flex items-center gap-1 rounded-lg px-2 py-0.5 text-[10px] text-slate-200"
            >
              <Upload className="h-3 w-3" aria-hidden="true" />
              Importar
            </button>
          </div>
        </div>

        <div className="mb-1.5 flex flex-wrap gap-1">
          {config.game_profiles.map((rule) => (
            <span
              key={rule.id}
              className="inline-flex items-center gap-1 rounded-md border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-[9px] text-slate-300"
            >
              {rule.name} · <span className="text-cyan-300">{rule.effect_mode}</span>
            </span>
          ))}
        </div>

        <textarea
          aria-label="JSON de perfil SenseHue"
          data-testid="profile-json-input"
          rows={2}
          placeholder="Pulsa Exportar para copiar tu configuración o pega un JSON .sensehue aquí..."
          value={profileJson}
          onChange={(e) => setProfileJson(e.target.value)}
          className="font-mono-tabular w-full resize-none rounded-lg border border-white/10 bg-slate-950/35 p-1.5 text-[10px] text-slate-200 placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
        />
      </div>
    </section>
  );
}
