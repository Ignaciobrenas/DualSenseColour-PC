import { useState } from 'react';
import { AlertCircle, Sliders, Sparkles, X } from 'lucide-react';
import { ColorControls } from './components/ColorControls';
import { DiagnosticsPanel } from './components/DiagnosticsPanel';
import { DualSensePreview } from './components/DualSensePreview';
import { TermsModal } from './components/TermsModal';
import { TitleBar } from './components/TitleBar';
import { useController } from './hooks/useController';
import { rgbToHex } from './types/controller';

const TERMS_STORAGE_KEY = 'sensehue_terms_accepted_v2';

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
    clearError,
    updateColor,
    applyPreset,
    turnOffLightbar,
    updatePlayerAndMic,
    updatePreferences,
    updateEffectMode,
    updateAdaptiveTriggers,
    exportProfileJson,
    importProfileJson,
    resyncController,
    resetToDefaults,
  } = useController();

  const [isTermsOpen, setIsTermsOpen] = useState<boolean>(shouldOpenTermsOnLaunch);
  const [activeTab, setActiveTab] = useState<'lighting' | 'diagnostics'>('lighting');

  const handleAcceptTerms = () => {
    try {
      window.localStorage.setItem(TERMS_STORAGE_KEY, 'true');
    } catch {
      // Ignore restricted storage environments
    }
    setIsTermsOpen(false);
  };

  const activeHex = rgbToHex(config.color);

  return (
    <main className="glass-shell flex h-screen w-screen flex-col justify-between overflow-hidden rounded-[22px] text-slate-100">
      <TitleBar
        status={status}
        onOpenTerms={() => setIsTermsOpen(true)}
        onResync={() => void resyncController()}
      />

      <div className="flex flex-1 flex-col justify-between overflow-y-auto px-3.5 pt-1.5 pb-3">
        {/* Dual-Mode Suite Switcher */}
        <div
          role="tablist"
          aria-label="Secciones de SenseHue"
          className="mb-1.5 grid grid-cols-2 gap-1.5 rounded-xl border border-white/[0.07] bg-slate-950/25 p-1 backdrop-blur-md"
        >
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'lighting'}
            data-testid="tab-lighting"
            onClick={() => setActiveTab('lighting')}
            className={`inline-flex items-center justify-center gap-1.5 rounded-lg py-1 text-[11px] font-medium transition-all ${
              activeTab === 'lighting'
                ? 'bg-white/14 text-white shadow-[0_0_12px_rgba(0,229,255,0.16)]'
                : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            <Sparkles className="h-3 w-3 text-cyan-400" aria-hidden="true" />
            Iluminación & Efectos
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'diagnostics'}
            data-testid="tab-diagnostics"
            onClick={() => setActiveTab('diagnostics')}
            className={`inline-flex items-center justify-center gap-1.5 rounded-lg py-1 text-[11px] font-medium transition-all ${
              activeTab === 'diagnostics'
                ? 'bg-white/14 text-white shadow-[0_0_12px_rgba(168,85,247,0.18)]'
                : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            <Sliders className="h-3 w-3 text-violet-400" aria-hidden="true" />
            Diagnóstico & Pro
          </button>
        </div>

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
          <p className="font-mono-tabular -mt-1 text-[11px] tracking-wider text-slate-400">
            LIGHTBAR · <span className="text-slate-200">{activeHex}</span> · {config.brightness}%
          </p>
        </div>

        {/* Optional HID Warning Toast */}
        {error && (
          <div
            role="alert"
            className="mb-2 flex items-center justify-between gap-2 rounded-xl border border-rose-500/35 bg-rose-500/15 px-3 py-1.5 text-[11px] text-rose-200"
          >
            <span className="inline-flex items-center gap-1.5 truncate">
              <AlertCircle className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
              {error}
            </span>
            <button
              type="button"
              aria-label="Cerrar aviso"
              onClick={clearError}
              className="rounded p-0.5 hover:bg-white/10"
            >
              <X className="h-3 w-3" aria-hidden="true" />
            </button>
          </div>
        )}

        {/* Active Suite Panel */}
        {activeTab === 'lighting' ? (
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
        ) : (
          <DiagnosticsPanel
            status={status}
            config={config}
            onPreferencesChange={(patch) => void updatePreferences(patch)}
            onTriggersChange={(left, right) => void updateAdaptiveTriggers(left, right)}
            onExportProfile={exportProfileJson}
            onImportProfile={importProfileJson}
          />
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
