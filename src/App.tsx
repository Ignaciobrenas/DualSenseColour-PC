import { useState } from 'react';
import {
  AlertCircle,
  Battery,
  BatteryCharging,
  BatteryWarning,
  Compass,
  Cpu,
  RefreshCw,
  Sliders,
  Sparkles,
  X,
} from 'lucide-react';
import { ColorControls } from './components/ColorControls';
import { ControllerSwitcher } from './components/ControllerSwitcher';
import { DiagnosticsPanel } from './components/DiagnosticsPanel';
import { DiscoverPanel } from './components/DiscoverPanel';
import { DualSensePreview } from './components/DualSensePreview';
import { EcosystemPanel } from './components/EcosystemPanel';
import { TermsModal } from './components/TermsModal';
import { TitleBar } from './components/TitleBar';
import { useController } from './hooks/useController';
import { formatFriendlyError, rgbToHex } from './types/controller';

const TERMS_STORAGE_KEY = 'sensehue_terms_accepted_v2';

type ActiveTab = 'lighting' | 'diagnostics' | 'ecosystem' | 'discover';

function shouldOpenTermsOnLaunch(): boolean {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
    return false;
  }
  try {
    return window.localStorage.getItem(TERMS_STORAGE_KEY) !== 'true';
  } catch {
    return true;
  }
}

export function App() {
  const {
    status,
    config,
    presets,
    error,
    lowBatteryAlert,
    autostartEnabled,
    clearError,
    dismissLowBatteryAlert,
    updateColor,
    applyPreset,
    turnOffLightbar,
    updatePlayerAndMic,
    updatePreferences,
    updateEffectMode,
    updateAdaptiveTriggers,
    exportProfileJson,
    importProfileJson,
    installCommunityProfile,
    testOpenRgbSync,
    testWledSync,
    toggleAutostart,
    selectController,
    resyncController,
    resetToDefaults,
  } = useController();

  const [isTermsOpen, setIsTermsOpen] = useState<boolean>(shouldOpenTermsOnLaunch);
  const [activeTab, setActiveTab] = useState<ActiveTab>('lighting');
  const [dismissedLowAt, setDismissedLowAt] = useState<number | null>(null);

  const handleAcceptTerms = () => {
    try {
      window.localStorage.setItem(TERMS_STORAGE_KEY, 'true');
    } catch {
      // Ignore restricted storage environments
    }
    setIsTermsOpen(false);
  };

  const activeHex = rgbToHex(config.color);
  const friendlyError = error ? formatFriendlyError(error) : null;
  const batteryPercent =
    typeof status.battery_percent === 'number' ? status.battery_percent : null;
  const isCharging = status.charging_state === 'charging';
  const isFullBattery = status.charging_state === 'full';
  const isDischarging = !isCharging && !isFullBattery;
  const batteryToneClass =
    batteryPercent === null
      ? 'border-white/10 bg-white/[0.04] text-slate-300'
      : batteryPercent >= 50
        ? 'border-emerald-400/25 bg-emerald-500/12 text-emerald-300'
        : batteryPercent > 20
          ? 'border-amber-400/25 bg-amber-500/12 text-amber-300'
          : 'border-rose-400/35 bg-rose-500/18 text-rose-200';

  const shouldShowInlineLowAlert =
    lowBatteryAlert !== null ||
    (config.low_battery_alerts &&
      status.connected &&
      isDischarging &&
      batteryPercent !== null &&
      batteryPercent <= 20 &&
      dismissedLowAt !== batteryPercent);

  return (
    <main className="glass-shell flex h-screen w-screen flex-col justify-between overflow-hidden rounded-[22px] text-slate-100">
      <TitleBar
        status={status}
        onOpenTerms={() => setIsTermsOpen(true)}
        onResync={() => void resyncController()}
      />

      <div className="flex flex-1 flex-col justify-between overflow-y-auto px-3.5 pt-1.5 pb-3">
        {/* 4-Mode Suite Switcher */}
        <div
          role="tablist"
          aria-label="Secciones de SenseHue"
          className="mb-1.5 grid grid-cols-4 gap-1 rounded-xl border border-white/[0.08] bg-slate-950/30 p-1 backdrop-blur-md"
        >
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'lighting'}
            data-testid="tab-lighting"
            onClick={() => setActiveTab('lighting')}
            className={`inline-flex items-center justify-center gap-1 rounded-lg py-1 text-[10.5px] font-medium transition-all ${
              activeTab === 'lighting'
                ? 'bg-white/14 text-white shadow-[0_0_12px_rgba(0,229,255,0.16)]'
                : 'text-slate-300 hover:text-white'
            }`}
          >
            <Sparkles className="h-3 w-3 text-cyan-400" aria-hidden="true" />
            Luz
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'diagnostics'}
            data-testid="tab-diagnostics"
            onClick={() => setActiveTab('diagnostics')}
            className={`inline-flex items-center justify-center gap-1 rounded-lg py-1 text-[10.5px] font-medium transition-all ${
              activeTab === 'diagnostics'
                ? 'bg-white/14 text-white shadow-[0_0_12px_rgba(168,85,247,0.18)]'
                : 'text-slate-300 hover:text-white'
            }`}
          >
            <Sliders className="h-3 w-3 text-violet-400" aria-hidden="true" />
            Pro
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'ecosystem'}
            data-testid="tab-ecosystem"
            onClick={() => setActiveTab('ecosystem')}
            className={`inline-flex items-center justify-center gap-1 rounded-lg py-1 text-[10.5px] font-medium transition-all ${
              activeTab === 'ecosystem'
                ? 'bg-white/14 text-white shadow-[0_0_12px_rgba(16,185,129,0.18)]'
                : 'text-slate-300 hover:text-white'
            }`}
          >
            <Cpu className="h-3 w-3 text-emerald-400" aria-hidden="true" />
            Ecosistema
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'discover'}
            data-testid="tab-discover"
            onClick={() => setActiveTab('discover')}
            className={`inline-flex items-center justify-center gap-1 rounded-lg py-1 text-[10.5px] font-medium transition-all ${
              activeTab === 'discover'
                ? 'bg-white/14 text-white shadow-[0_0_12px_rgba(56,189,248,0.2)]'
                : 'text-slate-300 hover:text-white'
            }`}
          >
            <Compass className="h-3 w-3 text-sky-400" aria-hidden="true" />
            Discover
          </button>
        </div>

        {/* Multi-Controller Switcher (USB & Bluetooth Simultaneous Configuration) */}
        <ControllerSwitcher
          status={status}
          config={config}
          onSelect={(id, syncAll) => void selectController(id, syncAll)}
        />

        {/* Hero Controller Visualizer */}
        <div className="relative flex flex-1 flex-col items-center justify-center">
          <DualSensePreview
            r={config.color.r}
            g={config.color.g}
            b={config.color.b}
            brightness={config.brightness}
            playerLeds={config.player_leds}
            micLed={config.mic_led}
            connected={status.connected}
          />
          <p className="font-mono-tabular -mt-1 text-[11px] tracking-wider text-slate-300">
            LIGHTBAR · <span className="text-slate-100">{activeHex}</span> · {config.brightness}%
          </p>
          {status.connected && (
            <div
              data-testid="hero-battery-readout"
              className={`font-mono-tabular mt-1 inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[10px] font-medium ${batteryToneClass}`}
            >
              {isCharging ? (
                <BatteryCharging className="h-3 w-3 shrink-0" aria-hidden="true" />
              ) : (
                <Battery className="h-3 w-3 shrink-0" aria-hidden="true" />
              )}
              <span>
                Batería{' '}
                {batteryPercent !== null
                  ? `${batteryPercent}%${
                      isCharging
                        ? ' · Cargando'
                        : isFullBattery
                          ? ' · Completa'
                          : batteryPercent <= 5
                            ? ' · Crítica'
                            : batteryPercent <= 20
                              ? ' · Baja'
                              : ''
                    }`
                  : '· Leyendo...'}
              </span>
            </div>
          )}
        </div>

        {/* Low Battery Warning Toast */}
        {shouldShowInlineLowAlert && (
          <div
            role="status"
            data-testid="low-battery-banner"
            className="mb-1.5 flex items-center justify-between gap-2 rounded-xl border border-amber-400/35 bg-amber-950/45 px-2.5 py-1.5 text-[10.5px] text-amber-100 backdrop-blur-md"
          >
            <div className="flex min-w-0 items-center gap-2">
              <BatteryWarning className="h-3.5 w-3.5 shrink-0 text-amber-300" aria-hidden="true" />
              <p className="truncate font-medium">
                {lowBatteryAlert?.body ??
                  `Tu mando se está quedando sin batería (${batteryPercent}%). Conéctalo por USB pronto.`}
              </p>
            </div>
            <button
              type="button"
              aria-label="Ocultar aviso de batería baja"
              onClick={() => {
                dismissLowBatteryAlert();
                setDismissedLowAt(batteryPercent);
              }}
              className="rounded p-0.5 text-amber-200/80 hover:bg-white/10 hover:text-white"
            >
              <X className="h-3 w-3" aria-hidden="true" />
            </button>
          </div>
        )}

        {/* Friendly HID Warning Toast */}
        {friendlyError && (
          <div
            role="alert"
            data-testid="friendly-error-toast"
            className="mb-2 flex items-center justify-between gap-2 rounded-xl border border-rose-500/35 bg-rose-950/55 px-3 py-2 text-[11px] text-rose-100 backdrop-blur-md"
          >
            <div className="flex min-w-0 items-start gap-2">
              <AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-rose-300" aria-hidden="true" />
              <div className="min-w-0">
                <p className="font-semibold text-rose-100">{friendlyError.title}</p>
                <p className="text-[10px] leading-snug text-rose-200/85">{friendlyError.detail}</p>
              </div>
            </div>
            <div className="flex shrink-0 items-center gap-1">
              <button
                type="button"
                data-testid="error-retry-btn"
                onClick={() => {
                  clearError();
                  void resyncController();
                }}
                className="inline-flex items-center gap-1 rounded-lg border border-rose-300/30 bg-rose-400/15 px-2 py-1 text-[10px] font-medium text-rose-100 transition-colors hover:bg-rose-400/25"
              >
                <RefreshCw className="h-3 w-3" aria-hidden="true" />
                Reintentar
              </button>
              <button
                type="button"
                aria-label="Cerrar aviso"
                onClick={clearError}
                className="rounded p-1 text-rose-200/80 hover:bg-white/10 hover:text-white"
              >
                <X className="h-3 w-3" aria-hidden="true" />
              </button>
            </div>
          </div>
        )}

        {/* Active Suite Panel */}
        {activeTab === 'lighting' && (
          <ColorControls
            config={config}
            presets={presets}
            onColorChange={updateColor}
            onSelectPreset={(id) => void applyPreset(id)}
            onTurnOff={() => void turnOffLightbar()}
            onPlayerAndMicChange={(leds, mic) => void updatePlayerAndMic(leds, mic)}
            onPreferencesChange={(patch) => void updatePreferences(patch)}
            onEffectModeChange={(mode) => void updateEffectMode(mode)}
          />
        )}
        {activeTab === 'diagnostics' && (
          <DiagnosticsPanel
            status={status}
            config={config}
            onPreferencesChange={(patch) => void updatePreferences(patch)}
            onTriggersChange={(left, right) => void updateAdaptiveTriggers(left, right)}
            onExportProfile={exportProfileJson}
            onImportProfile={importProfileJson}
          />
        )}
        {activeTab === 'ecosystem' && (
          <EcosystemPanel
            config={config}
            autostartEnabled={autostartEnabled}
            onPreferencesChange={(patch) => void updatePreferences(patch)}
            onToggleAutostart={toggleAutostart}
            onTestOpenRgb={testOpenRgbSync}
            onTestWled={testWledSync}
          />
        )}
        {activeTab === 'discover' && (
          <DiscoverPanel onInstallProfile={installCommunityProfile} />
        )}
      </div>

      <TermsModal
        open={isTermsOpen}
        status={status}
        onClose={() => setIsTermsOpen(false)}
        onAccept={handleAcceptTerms}
        onResync={() => void resyncController()}
        onResetDefaults={() => void resetToDefaults()}
      />
    </main>
  );
}

export default App;
