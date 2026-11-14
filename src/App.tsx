import { useState } from 'react';
import { AlertCircle, X } from 'lucide-react';
import { ColorControls } from './components/ColorControls';
import { DualSensePreview } from './components/DualSensePreview';
import { TermsModal } from './components/TermsModal';
import { TitleBar } from './components/TitleBar';
import { useController } from './hooks/useController';
import { rgbToHex } from './types/controller';

const TERMS_STORAGE_KEY = 'dualsensecolour_terms_accepted_v1';

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
    resyncController,
    resetToDefaults,
  } = useController();

  const [isTermsOpen, setIsTermsOpen] = useState<boolean>(shouldOpenTermsOnLaunch);

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

      <div className="flex flex-1 flex-col justify-between px-3.5 pt-1.5 pb-3">
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

        {/* Controls Dock */}
        <ColorControls
          config={config}
          presets={presets}
          onColorChange={updateColor}
          onSelectPreset={(id) => void applyPreset(id)}
          onTurnOff={() => void turnOffLightbar()}
          onPlayerAndMicChange={(leds, mic) => void updatePlayerAndMic(leds, mic)}
          onPreferencesChange={(patch) => void updatePreferences(patch)}
        />
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
