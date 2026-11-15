import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  BUILTIN_PRESETS,
  DEFAULT_CONFIG,
  DEFAULT_STATUS,
  parseHexColor,
  type AppConfig,
  type BatteryAlertPayload,
  type ColorPreset,
  type CommunityProfile,
  type ConnectionStatus,
  type LightingEffectMode,
  type MicLedState,
  type RgbColor,
  type TriggerPreset,
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
  lowBatteryAlert: BatteryAlertPayload | null;
  autostartEnabled: boolean;
  clearError: () => void;
  dismissLowBatteryAlert: () => void;
  updateColor: (color: RgbColor, brightness: number, persist: boolean) => void;
  applyPreset: (presetId: string) => Promise<void>;
  turnOffLightbar: () => Promise<void>;
  updatePlayerAndMic: (playerLeds: number, micLed: MicLedState) => Promise<void>;
  updatePreferences: (patch: Partial<AppConfig>) => Promise<void>;
  updateEffectMode: (mode: LightingEffectMode) => Promise<void>;
  updateAdaptiveTriggers: (left: TriggerPreset, right: TriggerPreset) => Promise<void>;
  exportProfileJson: () => Promise<string>;
  importProfileJson: (json: string) => Promise<void>;
  installCommunityProfile: (profile: CommunityProfile | string) => Promise<void>;
  testOpenRgbSync: (addr: string) => Promise<string>;
  testWledSync: (addr: string) => Promise<string>;
  toggleAutostart: () => Promise<void>;
  resyncController: () => Promise<void>;
  resetToDefaults: () => Promise<void>;
}

export function useController(): ControllerHook {
  const [status, setStatus] = useState<ConnectionStatus>(getInitialStatus);
  const [config, setConfig] = useState<AppConfig>(DEFAULT_CONFIG);
  const [presets, setPresets] = useState<ColorPreset[]>(BUILTIN_PRESETS);
  const [error, setError] = useState<string | null>(null);
  const [lowBatteryAlert, setLowBatteryAlert] = useState<BatteryAlertPayload | null>(null);
  const [autostartEnabled, setAutostartEnabled] = useState<boolean>(false);

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
    let unlistenBatteryAlert: UnlistenFn | undefined;

    const bootstrap = async () => {
      try {
        const [initialStatus, initialConfig, loadedPresets, initialAutostart] = await Promise.all([
          invoke<ConnectionStatus>('get_controller_status'),
          invoke<AppConfig>('get_config'),
          invoke<ColorPreset[]>('get_presets'),
          invoke<boolean>('get_autostart_enabled').catch(() => false),
        ]);
        if (!cancelled) {
          setStatus(initialStatus);
          setConfig(initialConfig);
          setAutostartEnabled(initialAutostart);
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
          if (
            event.payload.charging_state === 'charging' ||
            event.payload.charging_state === 'full' ||
            (typeof event.payload.battery_percent === 'number' &&
              event.payload.battery_percent > 25)
          ) {
            setLowBatteryAlert(null);
          }
        }
      });

      unlistenColor = await listen<AppConfig>('lightbar-color-changed', (event) => {
        if (!cancelled) {
          setConfig(event.payload);
        }
      });

      unlistenBatteryAlert = await listen<BatteryAlertPayload>('battery-low-alert', (event) => {
        if (!cancelled) {
          setLowBatteryAlert(event.payload);
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
      unlistenBatteryAlert?.();
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
    async (patch: Partial<AppConfig>) => {
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

  const updateEffectMode = useCallback(async (mode: LightingEffectMode) => {
    setConfig((prev) => ({ ...prev, effect_mode: mode }));
    if (!isTauriRuntime()) {
      return;
    }

    try {
      const updated = await invoke<AppConfig>('set_effect_mode', { mode });
      setConfig(updated);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const updateAdaptiveTriggers = useCallback(
    async (left: TriggerPreset, right: TriggerPreset) => {
      setConfig((prev) => ({ ...prev, left_trigger: left, right_trigger: right }));
      if (!isTauriRuntime()) {
        return;
      }

      try {
        const updated = await invoke<AppConfig>('set_adaptive_triggers', { left, right });
        setConfig(updated);
        setError(null);
      } catch (err) {
        setError(String(err));
      }
    },
    [],
  );

  const exportProfileJson = useCallback(async (): Promise<string> => {
    if (!isTauriRuntime()) {
      return JSON.stringify(config, null, 2);
    }
    return invoke<string>('export_profile_json');
  }, [config]);

  const importProfileJson = useCallback(async (json: string) => {
    try {
      const parsed = JSON.parse(json) as Partial<AppConfig>;
      if (!isTauriRuntime()) {
        setConfig((prev) => ({ ...prev, ...parsed }));
        setError(null);
        return;
      }
      const updated = await invoke<AppConfig>('import_profile_json', { json });
      setConfig(updated);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const clearError = useCallback(() => setError(null), []);
  const dismissLowBatteryAlert = useCallback(() => setLowBatteryAlert(null), []);

  const installCommunityProfile = useCallback(async (profileOrJson: CommunityProfile | string) => {
    const rawJson =
      typeof profileOrJson === 'string' ? profileOrJson : JSON.stringify(profileOrJson);

    if (!isTauriRuntime()) {
      const parsed =
        typeof profileOrJson === 'string'
          ? (JSON.parse(profileOrJson) as CommunityProfile)
          : profileOrJson;
      const parsedColor = parseHexColor(parsed.color);
      setConfig((prev) => ({
        ...prev,
        color: parsedColor ?? prev.color,
        brightness: Math.max(0, Math.min(100, parsed.brightness)),
        player_leds: Math.max(0, Math.min(5, parsed.player_leds)),
        left_trigger: parsed.left_trigger,
        right_trigger: parsed.right_trigger,
        preset_id: null,
      }));
      setError(null);
      return;
    }

    try {
      const saved = await invoke<AppConfig>('validate_and_install_community_profile', {
        rawJson,
      });
      setConfig(saved);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const testOpenRgbSync = useCallback(async (addr: string): Promise<string> => {
    if (!isTauriRuntime()) {
      return `OpenRGB listo (${addr}) en modo vista previa`;
    }
    const count = await invoke<number>('test_openrgb_sync', { addr });
    return `Conectado · ${count} dispositivo${count === 1 ? '' : 's'} RGB sincronizados`;
  }, []);

  const testWledSync = useCallback(async (addr: string): Promise<string> => {
    if (!isTauriRuntime()) {
      return `WLED DRGB enviado (${addr}) en modo vista previa`;
    }
    const bytes = await invoke<number>('test_wled_sync', { addr });
    return `Frame DRGB enviado (${bytes} bytes) a ${addr}`;
  }, []);

  const toggleAutostart = useCallback(async () => {
    const next = !autostartEnabled;
    setAutostartEnabled(next);
    if (!isTauriRuntime()) {
      return;
    }
    try {
      const updated = await invoke<boolean>('set_autostart_enabled', { enabled: next });
      setAutostartEnabled(updated);
      setError(null);
    } catch (err) {
      setAutostartEnabled(!next);
      setError(String(err));
    }
  }, [autostartEnabled]);

  const resyncController = useCallback(async () => {
    if (!isTauriRuntime()) {
      return;
    }

    try {
      const freshStatus = await invoke<ConnectionStatus>('resync_controller');
      setStatus(freshStatus);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const resetToDefaults = useCallback(async () => {
    setConfig(DEFAULT_CONFIG);
    if (!isTauriRuntime()) {
      return;
    }

    try {
      const saved = await invoke<AppConfig>('save_config', { config: DEFAULT_CONFIG });
      setConfig(saved);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  return {
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
    resyncController,
    resetToDefaults,
  };
}
