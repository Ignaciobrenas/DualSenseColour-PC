import { useEffect, useRef, useState } from 'react';
import { Mic, Power, Sparkles, Sun, Users } from 'lucide-react';
import {
  hsvToRgb,
  parseHexColor,
  rgbToHex,
  rgbToHsv,
  type AppConfig,
  type ColorPreset,
  type LightingEffectMode,
  type MicLedState,
  type RgbColor,
} from '../types/controller';

interface ColorControlsProps {
  config: AppConfig;
  presets: ColorPreset[];
  onColorChange: (color: RgbColor, brightness: number, persist: boolean) => void;
  onSelectPreset: (presetId: string) => void;
  onTurnOff: () => void;
  onPlayerAndMicChange: (playerLeds: number, micLed: MicLedState) => void;
  onPreferencesChange: (patch: Partial<AppConfig>) => void;
  onEffectModeChange?: (mode: LightingEffectMode) => void;
}

const EFFECT_MODES: { id: LightingEffectMode; label: string }[] = [
  { id: 'static', label: 'Estático' },
  { id: 'breathing', label: 'Breathing' },
  { id: 'spectrum', label: 'Spectrum' },
  { id: 'battery_smart', label: 'Batería' },
  { id: 'input_reactive', label: 'Reactivo' },
  { id: 'audio_beat', label: 'Audio' },
  { id: 'screen_ambilight', label: 'Ambilight' },
];

const MIC_OPTIONS: { value: MicLedState; label: string }[] = [
  { value: 'off', label: 'Off' },
  { value: 'on', label: 'On' },
  { value: 'pulse', label: 'Pulse' },
];

export function ColorControls({
  config,
  presets,
  onColorChange,
  onSelectPreset,
  onTurnOff,
  onPlayerAndMicChange,
  onPreferencesChange,
  onEffectModeChange,
}: ColorControlsProps) {
  const [hsv, setHsv] = useState(() => rgbToHsv(config.color));
  const [hexDraft, setHexDraft] = useState(() => rgbToHex(config.color));
  const svPadRef = useRef<HTMLDivElement | null>(null);
  const draggingSvRef = useRef(false);

  useEffect(() => {
    const incomingHex = rgbToHex(config.color);
    setHexDraft(incomingHex);
    setHsv((prev) => {
      const next = rgbToHsv(config.color);
      // Preserve last hue when brightness/saturation hits black/white boundary
      return next.s < 1 || next.v < 1 ? { ...next, h: prev.h } : next;
    });
  }, [config.color]);

  const commitHsv = (nextH: number, nextS: number, nextV: number, persist: boolean) => {
    const nextHsv = { h: nextH, s: nextS, v: nextV };
    setHsv(nextHsv);
    const rgb = hsvToRgb(nextHsv);
    setHexDraft(rgbToHex(rgb));
    const effectiveBrightness = config.brightness === 0 && !persist ? 100 : config.brightness;
    onColorChange(rgb, effectiveBrightness, persist);
  };

  const handleSvPointer = (clientX: number, clientY: number, persist: boolean) => {
    const rect = svPadRef.current?.getBoundingClientRect();
    if (!rect || rect.width === 0 || rect.height === 0) {
      return;
    }
    const s = Math.max(0, Math.min(100, ((clientX - rect.left) / rect.width) * 100));
    const v = Math.max(0, Math.min(100, (1 - (clientY - rect.top) / rect.height) * 100));
    commitHsv(hsv.h, s, v, persist);
  };

  const handleHexInputChange = (value: string) => {
    setHexDraft(value.toUpperCase());
    const parsed = parseHexColor(value);
    if (parsed) {
      onColorChange(parsed, config.brightness === 0 ? 100 : config.brightness, true);
    }
  };

  const currentHex = rgbToHex(config.color);
  const hueColor = `hsl(${Math.round(hsv.h)}, 100%, 50%)`;

  return (
    <section aria-label="Controles de color y LEDs" className="flex w-full min-w-0 flex-col gap-2">
      {/* Interactive Color Spectrum & HEX/RGB Card */}
      <div className="glass-card w-full min-w-0 rounded-2xl p-2.5">
        <div className="flex min-w-0 items-stretch gap-2.5">
          {/* 2D Saturation/Value Surface */}
          <div
            ref={svPadRef}
            role="application"
            aria-label="Selector interactivo de color"
            data-testid="sv-color-pad"
            onPointerDown={(e) => {
              draggingSvRef.current = true;
              e.currentTarget.setPointerCapture?.(e.pointerId);
              handleSvPointer(e.clientX, e.clientY, false);
            }}
            onPointerMove={(e) => {
              if (draggingSvRef.current) {
                handleSvPointer(e.clientX, e.clientY, false);
              }
            }}
            onPointerUp={(e) => {
              if (draggingSvRef.current) {
                draggingSvRef.current = false;
                handleSvPointer(e.clientX, e.clientY, true);
              }
            }}
            className="relative h-[82px] min-w-0 flex-1 cursor-crosshair overflow-hidden rounded-xl border border-white/15"
            style={{
              backgroundColor: hueColor,
              backgroundImage:
                'linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, transparent)',
            }}
          >
            <span
              aria-hidden="true"
              className="pointer-events-none absolute h-3.5 w-3.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_6px_rgba(0,0,0,0.85)]"
              style={{
                left: `${hsv.s}%`,
                top: `${100 - hsv.v}%`,
                backgroundColor: currentHex,
              }}
            />
          </div>

          {/* Color Readout & Quick Power Off */}
          <div className="flex w-[112px] shrink-0 flex-col justify-between">
            <div className="flex items-center gap-2">
              <span
                aria-hidden="true"
                className="h-7 w-7 shrink-0 rounded-lg border border-white/20 shadow-inner"
                style={{
                  backgroundColor: currentHex,
                  boxShadow:
                    config.brightness > 0
                      ? `0 0 12px rgba(${config.color.r}, ${config.color.g}, ${config.color.b}, 0.55)`
                      : 'none',
                }}
              />
              <input
                type="text"
                aria-label="Código hexadecimal HEX"
                data-testid="hex-input"
                maxLength={7}
                value={hexDraft}
                onChange={(e) => handleHexInputChange(e.target.value)}
                onBlur={() => setHexDraft(currentHex)}
                className="font-mono-tabular w-full rounded-lg border border-white/12 bg-slate-950/25 px-2 py-1 text-xs font-semibold uppercase text-slate-100 backdrop-blur-sm focus:border-sky-400/60 focus:outline-none"
              />
            </div>

            <div className="font-mono-tabular flex items-center justify-between text-[10px] text-slate-400">
              <span>R:{config.color.r}</span>
              <span>G:{config.color.g}</span>
              <span>B:{config.color.b}</span>
            </div>

            <button
              type="button"
              onClick={onTurnOff}
              data-testid="turn-off-btn"
              className="ps-action-tile group inline-flex items-center justify-center gap-1.5 rounded-lg px-2 py-1 text-[11px] font-medium text-slate-200 hover:border-rose-400/40 hover:text-rose-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-400"
            >
              <Power className="h-3 w-3 text-rose-400/85 transition-transform group-hover:scale-110" aria-hidden="true" />
              Apagar LED
            </button>
          </div>
        </div>

        {/* Full-spectrum Hue Slider */}
        <div className="mt-2.5 flex items-center gap-2.5">
          <Sparkles className="h-3.5 w-3.5 shrink-0 text-slate-400" aria-hidden="true" />
          <input
            type="range"
            aria-label="Tono del color"
            data-testid="hue-slider"
            min={0}
            max={359}
            value={Math.round(hsv.h)}
            onChange={(e) => commitHsv(Number(e.target.value), hsv.s, hsv.v, false)}
            onPointerUp={(e) =>
              commitHsv(Number((e.target as HTMLInputElement).value), hsv.s, hsv.v, true)
            }
            className="glass-slider"
            style={{
              background:
                'linear-gradient(to right, #ff0000 0%, #ffff00 17%, #00ff00 33%, #00ffff 50%, #0000ff 67%, #ff00ff 83%, #ff0000 100%)',
            }}
          />
        </div>
      </div>

      {/* Quick Curated Presets Row */}
      <div className="flex items-center justify-between gap-1.5 px-0.5" role="group" aria-label="Presets rápidos">
        {presets.map((preset) => {
          const isSelected = config.preset_id === preset.id;
          return (
            <button
              key={preset.id}
              type="button"
              title={preset.name}
              aria-label={`Aplicar preset ${preset.name}`}
              aria-pressed={isSelected}
              data-testid={`preset-${preset.id}`}
              onClick={() => onSelectPreset(preset.id)}
              className={`glass-pill group relative flex h-8 flex-1 items-center justify-center rounded-xl focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-400 ${
                isSelected ? 'border-white/40 bg-white/14' : ''
              }`}
            >
              <span
                className="h-4 w-4 rounded-full border border-white/25 transition-transform group-hover:scale-110"
                style={{
                  backgroundColor: preset.id === 'off' ? '#181c26' : preset.hex,
                  boxShadow:
                    isSelected && preset.id !== 'off' ? `0 0 10px ${preset.hex}` : 'none',
                }}
              />
            </button>
          );
        })}
      </div>

      {/* Brightness & Hardware Secondary Indicators Card */}
      <div className="glass-card flex w-full min-w-0 flex-col gap-2 rounded-2xl p-2.5">
        {/* Brightness Slider */}
        <div className="flex min-w-0 items-center gap-2">
          <Sun className="h-3.5 w-3.5 shrink-0 text-amber-300/80" aria-hidden="true" />
          <span className="shrink-0 text-[11px] font-medium text-slate-300">Brillo</span>
          <input
            type="range"
            aria-label="Brillo de la barra LED"
            data-testid="brightness-slider"
            min={0}
            max={100}
            value={config.brightness}
            onChange={(e) => onColorChange(config.color, Number(e.target.value), false)}
            onPointerUp={(e) =>
              onColorChange(
                config.color,
                Number((e.target as HTMLInputElement).value),
                true,
              )
            }
            className="glass-slider min-w-0 flex-1"
            style={{
              background: `linear-gradient(to right, ${currentHex} 0%, ${currentHex} ${config.brightness}%, rgba(255,255,255,0.12) ${config.brightness}%, rgba(255,255,255,0.12) 100%)`,
            }}
          />
          <span
            data-testid="brightness-value"
            className="font-mono-tabular w-8 shrink-0 text-right text-[11px] font-semibold text-slate-200"
          >
            {config.brightness}%
          </span>
        </div>

        {/* Player Indicator LEDs + Mic Mute LED */}
        <div className="flex min-w-0 items-center justify-between gap-2 border-t border-white/[0.06] pt-2">
          <div className="flex min-w-0 items-center gap-1.5">
            <span className="inline-flex shrink-0 items-center gap-1 text-[10px] text-slate-400">
              <Users className="h-3 w-3" aria-hidden="true" />
              LED
            </span>
            <div className="flex items-center gap-0.5 rounded-lg border border-white/[0.07] bg-slate-950/20 p-0.5 backdrop-blur-sm">
              {[0, 1, 2, 3, 4, 5].map((count) => (
                <button
                  key={count}
                  type="button"
                  aria-label={`${count} LEDs de jugador`}
                  aria-pressed={config.player_leds === count}
                  data-testid={`player-led-${count}`}
                  onClick={() => onPlayerAndMicChange(count, config.mic_led)}
                  className={`font-mono-tabular h-5 w-[18px] rounded-md text-[10px] font-semibold transition-colors ${
                    config.player_leds === count
                      ? 'bg-white/20 text-white'
                      : 'text-slate-400 hover:text-slate-200'
                  }`}
                >
                  {count}
                </button>
              ))}
            </div>
          </div>

          <div className="flex min-w-0 flex-1 items-center justify-end gap-1.5">
            <span className="inline-flex shrink-0 items-center gap-1 text-[10px] text-slate-400">
              <Mic className="h-3 w-3" aria-hidden="true" />
              Mic
            </span>
            <div className="flex max-w-[132px] flex-1 items-center gap-0.5 rounded-lg border border-white/[0.07] bg-slate-950/20 p-0.5 backdrop-blur-sm">
              {MIC_OPTIONS.map((opt) => (
                <button
                  key={opt.value}
                  type="button"
                  aria-label={`LED de micrófono ${opt.label}`}
                  aria-pressed={config.mic_led === opt.value}
                  data-testid={`mic-led-${opt.value}`}
                  onClick={() => onPlayerAndMicChange(config.player_leds, opt.value)}
                  className={`h-5 flex-1 rounded-md px-1 text-[10px] font-medium transition-colors ${
                    config.mic_led === opt.value
                      ? 'bg-amber-400/25 text-amber-200'
                      : 'text-slate-400 hover:text-slate-200'
                  }`}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Dynamic Effects Mode Strip */}
      <div
        className="flex flex-wrap items-center gap-1 rounded-xl border border-white/[0.06] bg-slate-950/20 p-1 backdrop-blur-sm"
        role="group"
        aria-label="Modo de efecto dinámico"
      >
        {EFFECT_MODES.map((mode) => {
          const active = config.effect_mode === mode.id;
          return (
            <button
              key={mode.id}
              type="button"
              aria-pressed={active}
              data-testid={`effect-mode-${mode.id}`}
              onClick={() => onEffectModeChange?.(mode.id)}
              className={`flex-1 rounded-lg px-1.5 py-1 text-[10px] font-medium whitespace-nowrap transition-all ${
                active
                  ? 'border border-cyan-400/40 bg-cyan-400/18 text-cyan-100 shadow-[0_0_10px_rgba(0,229,255,0.18)]'
                  : 'border border-transparent text-slate-400 hover:bg-white/[0.05] hover:text-slate-200'
              }`}
            >
              {mode.label}
            </button>
          );
        })}
      </div>

      {/* Behavior Preferences Pills */}
      <div className="grid grid-cols-3 gap-1.5">
        {(
          [
            {
              key: 'apply_on_connect',
              label: 'Auto-aplicar',
              active: config.apply_on_connect,
            },
            {
              key: 'keep_alive_steam',
              label: 'Steam Fix',
              active: config.keep_alive_steam,
            },
            {
              key: 'close_to_tray',
              label: 'Cerrar a tray',
              active: config.close_to_tray,
            },
          ] as const
        ).map((pref) => (
          <button
            key={pref.key}
            type="button"
            aria-pressed={pref.active}
            onClick={() => onPreferencesChange({ [pref.key]: !pref.active })}
            className={`ps-action-tile inline-flex items-center justify-center gap-1.5 rounded-xl px-2 py-1.5 text-[10px] font-medium ${
              pref.active
                ? 'border-sky-400/45 text-sky-100 shadow-[0_0_12px_rgba(56,189,248,0.18)]'
                : 'text-slate-400'
            }`}
          >
            <span
              aria-hidden="true"
              className={`h-1.5 w-1.5 rounded-full transition-all ${
                pref.active
                  ? 'bg-sky-400 shadow-[0_0_8px_#38bdf8]'
                  : 'bg-slate-600/70'
              }`}
            />
            {pref.label}
          </button>
        ))}
      </div>
    </section>
  );
}
