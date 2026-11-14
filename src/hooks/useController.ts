import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  BUILTIN_PRESETS,
  DEFAULT_CONFIG,
  DEFAULT_STATUS,
  type AppConfig,
  type ColorPreset,
  type ConnectionStatus,
  type MicLedState,
  type RgbColor,
} from '../types/controller';

const IPC_THROTTLE_MS = 22; // ~45Hz ceiling prevents Bluetooth HID output queue congestion

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function getInitialStatus(): ConnectionStatus {
  if (
    typeof window !== 'undefined' &&
    !isTauriRuntime() &&
    window.location.search.includes('demo=usb')
  ) {
    return {
      connected: true,
      transport: 'usb',
      model: 'DualSense',
      serial_number: null,
      product_name: 'DualSense Wireless Controller',
    };
  }
  return DEFAULT_STATUS;
}

export interface ControllerHook {
  status: ConnectionStatus;
  config: AppConfig;
  presets: ColorPreset[];
  error: string | null;
  clearError: () => void;
  updateColor: (color: RgbColor, brightness: number, persist: boolean) => void;
  applyPreset: (presetId: string) => Promise<void>;
  turnOffLightbar: () => Promise<void>;
  updatePlayerAndMic: (playerLeds: number, micLed: MicLedState) => Promise<void>;
  updatePreferences: (patch: Partial<Pick<AppConfig, 'apply_on_connect' | 'keep_alive_steam' | 'close_to_tray'>>) => Promise<void>;
}

export function useController(): ControllerHook {
  const [status, setStatus] = useState<ConnectionStatus>(getInitialStatus);
  const [config, setConfig] = useState<AppConfig>(DEFAULT_CONFIG);
  const [presets, setPresets] = useState<ColorPreset[]>(BUILTIN_PRESETS);
  const [error, setError] = useState<string | null>(null);

  const lastSentAtRef = useRef<number>(0);
  const pendingTimerRef = useRef<number | null>(null);
  const latestPayloadRef = useRef<{ color: RgbColor; brightness: number } | null>(null);

  useEffect(() => {
    document.documentElement.style.setProperty(
      '--accent-dynamic',
      `rgb(${config.color.r}, ${config.color.g}, ${config.color.b})`,
    );
    document.documentElement.style.setProperty(
      '--accent-rgb',
      `${config.color.r}, ${config.color.g}, ${config.color.b}`,
    );
  }, [config.color]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let cancelled = false;
    let unlistenStatus: UnlistenFn | undefined;
    let unlistenColor: UnlistenFn | undefined;

    const bootstrap = async () => {
      try {
        const [initialStatus, initialConfig, loadedPresets] = await Promise.all([
          invoke<ConnectionStatus>('get_controller_status'),
          invoke<AppConfig>('get_config'),
          invoke<ColorPreset[]>('get_presets'),
        ]);
        if (!cancelled) {
          setStatus(initialStatus);
          setConfig(initialConfig);
          if (loadedPresets.length > 0) {
            setPresets(loadedPresets);
          }
        }
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      }

      unlistenStatus = await listen<ConnectionStatus>('controller-status-changed', (event) => {
        if (!cancelled) {
          setStatus(event.payload);
        }
      });

      unlistenColor = await listen<AppConfig>('lightbar-color-changed', (event) => {
        if (!cancelled) {
          setConfig(event.payload);
        }
      });
    };

    void bootstrap();

    return () => {
      cancelled = true;
      if (pendingTimerRef.current !== null) {
        window.clearTimeout(pendingTimerRef.current);
      }
      unlistenStatus?.();
      unlistenColor?.();
    };
  }, []);

  const dispatchLightbarIpc = useCallback(
    (color: RgbColor, brightness: number, persist: boolean) => {
      if (!isTauriRuntime()) {
        return;
      }

      void invoke<AppConfig>('set_lightbar_color', {
        r: color.r,
        g: color.g,
        b: color.b,
        brightness,
        persist,
      })
        .then(() => {
          setError(null);
        })
        .catch((err: unknown) => {
          setError(String(err));
        });
    },
    [],
  );

  const updateColor = useCallback(
    (color: RgbColor, brightness: number, persist: boolean) => {
      const clampedBrightness = Math.max(0, Math.min(100, Math.round(brightness)));
      setConfig((prev) => ({
        ...prev,
        color,
        brightness: clampedBrightness,
        preset_id: null,
      }));

      if (!isTauriRuntime()) {
        return;
      }

      if (persist) {
        if (pendingTimerRef.current !== null) {
          window.clearTimeout(pendingTimerRef.current);
          pendingTimerRef.current = null;
        }
        lastSentAtRef.current = performance.now();
        latestPayloadRef.current = null;
        dispatchLightbarIpc(color, clampedBrightness, true);
        return;
      }

      const now = performance.now();
      const elapsed = now - lastSentAtRef.current;
      latestPayloadRef.current = { color, brightness: clampedBrightness };

      if (elapsed >= IPC_THROTTLE_MS) {
        lastSentAtRef.current = now;
        dispatchLightbarIpc(color, clampedBrightness, false);
        latestPayloadRef.current = null;
      } else if (pendingTimerRef.current === null) {
        pendingTimerRef.current = window.setTimeout(() => {
          pendingTimerRef.current = null;
          if (latestPayloadRef.current) {
            const payload = latestPayloadRef.current;
            latestPayloadRef.current = null;
            lastSentAtRef.current = performance.now();
            dispatchLightbarIpc(payload.color, payload.brightness, false);
          }
        }, IPC_THROTTLE_MS - elapsed);
      }
    },
    [dispatchLightbarIpc],
  );

  const applyPreset = useCallback(
    async (presetId: string) => {
      const found = presets.find((p) => p.id === presetId);
      if (found) {
        setConfig((prev) => ({
          ...prev,
          color: found.color,
          preset_id: found.id,
          brightness: found.id === 'off' ? 0 : prev.brightness === 0 ? 100 : prev.brightness,
        }));
      }

      if (!isTauriRuntime()) {
        return;
      }

      try {
        const updated = await invoke<AppConfig>('apply_preset', { presetId });
        setConfig(updated);
        setError(null);
      } catch (err) {
        setError(String(err));
      }
    },
    [presets],
  );

  const turnOffLightbar = useCallback(async () => {
    setConfig((prev) => ({
      ...prev,
      color: { r: 0, g: 0, b: 0 },
      brightness: 0,
      preset_id: 'off',
    }));

    if (!isTauriRuntime()) {
      return;
    }

    try {
      const updated = await invoke<AppConfig>('turn_off_lightbar');
      setConfig(updated);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const updatePlayerAndMic = useCallback(
    async (playerLeds: number, micLed: MicLedState) => {
      setConfig((prev) => ({
        ...prev,
        player_leds: playerLeds,
        mic_led: micLed,
      }));

      if (!isTauriRuntime()) {
        return;
      }

      try {
        const updated = await invoke<AppConfig>('set_player_leds', { playerLeds, micLed });
        setConfig(updated);
        setError(null);
      } catch (err) {
        setError(String(err));
      }
    },
    [],
  );

  const updatePreferences = useCallback(
    async (
      patch: Partial<Pick<AppConfig, 'apply_on_connect' | 'keep_alive_steam' | 'close_to_tray'>>,
    ) => {
      let nextConfig: AppConfig = DEFAULT_CONFIG;
      setConfig((prev) => {
        nextConfig = { ...prev, ...patch };
        return nextConfig;
      });

      if (!isTauriRuntime()) {
        return;
      }

      try {
        const saved = await invoke<AppConfig>('save_config', { config: nextConfig });
        setConfig(saved);
        setError(null);
      } catch (err) {
        setError(String(err));
      }
    },
    [],
  );

  const clearError = useCallback(() => setError(null), []);

  return {
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
  };
}
