export type TransportType = 'usb' | 'bluetooth';
export type DeviceModel = 'DualSense' | 'DualSenseEdge';
export type MicLedState = 'off' | 'on' | 'pulse';
export type LightingEffectMode =
  | 'static'
  | 'breathing'
  | 'spectrum'
  | 'battery_smart'
  | 'input_reactive'
  | 'audio_beat'
  | 'screen_ambilight';
export type TriggerPreset =
  | 'off'
  | 'rigid'
  | 'weapon_click'
  | 'pulse'
  | 'hair_trigger'
  | 'dual_stage'
  | 'progressive_pedal';
export type ChargingState = 'discharging' | 'charging' | 'full' | { unknown: number };

export interface BatteryAlertPayload {
  level?: 'Twenty' | 'Ten' | 'Five';
  threshold_percent: number;
  battery_percent: number;
  title: string;
  body: string;
}

export interface CommunityProfile {
  name: string;
  author: string;
  color: string;
  brightness: number;
  player_leds: number;
  left_trigger: TriggerPreset;
  right_trigger: TriggerPreset;
}

export interface DiscoverShowcaseItem {
  id: string;
  description: string;
  profile: CommunityProfile;
}

export interface RegistryEntry {
  id: string;
  name: string;
  author: string;
  description: string;
  url: string;
}

export interface RegistryIndex {
  version: number;
  presets: RegistryEntry[];
}

export interface DualSenseTelemetry {
  battery_percent: number;
  charging_state: ChargingState;
  lx: number;
  ly: number;
  rx: number;
  ry: number;
  l2: number;
  r2: number;
  mute_pressed: boolean;
  ps_pressed: boolean;
}

export interface GameProfileRule {
  id: string;
  name: string;
  matcher: { process_name: string } | { steam_app_id: number };
  preset_id: string;
  effect_mode: LightingEffectMode;
}

export interface RgbColor {
  r: number;
  g: number;
  b: number;
}

export interface HsvColor {
  h: number;
  s: number;
  v: number;
}

export interface ColorPreset {
  id: string;
  name: string;
  color: RgbColor;
  hex: string;
}

export interface ConnectionStatus {
  connected: boolean;
  transport: TransportType | null;
  model: DeviceModel | null;
  serial_number: string | null;
  product_name: string | null;
  battery_percent?: number | null;
  charging_state?: ChargingState | null;
  telemetry?: DualSenseTelemetry | null;
  connected_count?: number;
}

export interface AppConfig {
  color: RgbColor;
  brightness: number;
  preset_id: string | null;
  player_leds: number;
  mic_led: MicLedState;
  apply_on_connect: boolean;
  keep_alive_steam: boolean;
  close_to_tray: boolean;
  effect_mode: LightingEffectMode;
  battery_bar_leds: boolean;
  sync_all_controllers: boolean;
  left_trigger: TriggerPreset;
  right_trigger: TriggerPreset;
  local_api_enabled: boolean;
  game_integrations_enabled: boolean;
  low_battery_alerts: boolean;
  openrgb_enabled: boolean;
  openrgb_addr: string;
  wled_enabled: boolean;
  wled_addr: string;
  gyro_indicator_enabled: boolean;
  mic_mute_sync_enabled: boolean;
  usb_audio_guard_enabled: boolean;
  game_profiles: GameProfileRule[];
}

export const BUILTIN_PRESETS: ColorPreset[] = [
  { id: 'classic-blue', name: 'Classic Blue', color: { r: 0, g: 67, b: 156 }, hex: '#00439C' },
  { id: 'crimson-red', name: 'Crimson Red', color: { r: 255, g: 23, b: 68 }, hex: '#FF1744' },
  { id: 'cyber-cyan', name: 'Cyber Cyan', color: { r: 0, g: 240, b: 255 }, hex: '#00F0FF' },
  { id: 'emerald', name: 'Emerald', color: { r: 0, g: 230, b: 118 }, hex: '#00E676' },
  { id: 'gold', name: 'Gold', color: { r: 255, g: 214, b: 0 }, hex: '#FFD600' },
  { id: 'violet', name: 'Violet', color: { r: 157, g: 78, b: 221 }, hex: '#9D4EDD' },
  { id: 'off', name: 'Apagado', color: { r: 0, g: 0, b: 0 }, hex: '#000000' },
];

export const DEFAULT_GAME_PROFILES: GameProfileRule[] = [
  {
    id: 'cyberpunk-2077',
    name: 'Cyberpunk 2077',
    matcher: { process_name: 'Cyberpunk2077.exe' },
    preset_id: 'cyber-yellow',
    effect_mode: 'audio_beat',
  },
  {
    id: 'elden-ring',
    name: 'Elden Ring',
    matcher: { process_name: 'eldenring.exe' },
    preset_id: 'amber-gold',
    effect_mode: 'breathing',
  },
  {
    id: 'alan-wake-2',
    name: 'Alan Wake 2',
    matcher: { process_name: 'AlanWake2.exe' },
    preset_id: 'crimson-pulse',
    effect_mode: 'input_reactive',
  },
];

export const CURATED_COMMUNITY_PRESETS: DiscoverShowcaseItem[] = [
  {
    id: 'apex-competitive',
    description: 'Hair-Trigger en L2/R2 y cian de alto contraste para respuesta FPS inmediata.',
    profile: {
      name: 'Apex Hair-Trigger',
      author: 'SenseHue Pro',
      color: '#00E5FF',
      brightness: 90,
      player_leds: 2,
      left_trigger: 'hair_trigger',
      right_trigger: 'hair_trigger',
    },
  },
  {
    id: 'simracing-f1',
    description: 'Pedal progresivo en frenada L2 y aceleración R2 con LED rojo competición.',
    profile: {
      name: 'F1 Telemetry Grip',
      author: 'PaddockLab',
      color: '#FF1744',
      brightness: 95,
      player_leds: 5,
      left_trigger: 'progressive_pedal',
      right_trigger: 'progressive_pedal',
    },
  },
  {
    id: 'night-city-cyber',
    description: 'Ámbar neón con doble etapa R2 para shooters tácticos y sesiones nocturnas.',
    profile: {
      name: 'Night City Dual-Stage',
      author: 'V_Deck',
      color: '#FFB300',
      brightness: 80,
      player_leds: 3,
      left_trigger: 'rigid',
      right_trigger: 'dual_stage',
    },
  },
  {
    id: 'oled-stealth',
    description: 'Violeta tenue optimizado para ahorro de batería y cero reflejos en monitor OLED.',
    profile: {
      name: 'OLED Battery Saver',
      author: 'LowGlow',
      color: '#9D4EDD',
      brightness: 35,
      player_leds: 1,
      left_trigger: 'off',
      right_trigger: 'off',
    },
  },
];

export const DEFAULT_CONFIG: AppConfig = {
  color: { r: 0, g: 67, b: 156 },
  brightness: 100,
  preset_id: 'classic-blue',
  player_leds: 1,
  mic_led: 'off',
  apply_on_connect: true,
  keep_alive_steam: false,
  close_to_tray: true,
  effect_mode: 'static',
  battery_bar_leds: false,
  sync_all_controllers: true,
  left_trigger: 'off',
  right_trigger: 'off',
  local_api_enabled: false,
  game_integrations_enabled: false,
  low_battery_alerts: true,
  openrgb_enabled: false,
  openrgb_addr: '127.0.0.1:6742',
  wled_enabled: false,
  wled_addr: '192.168.1.50:21324',
  gyro_indicator_enabled: false,
  mic_mute_sync_enabled: false,
  usb_audio_guard_enabled: false,
  game_profiles: DEFAULT_GAME_PROFILES,
};

export const DEFAULT_STATUS: ConnectionStatus = {
  connected: false,
  transport: null,
  model: null,
  serial_number: null,
  product_name: null,
  battery_percent: null,
  charging_state: null,
  telemetry: null,
  connected_count: 0,
};

export function clampByte(value: number): number {
  return Math.max(0, Math.min(255, Math.round(value)));
}

export function rgbToHex({ r, g, b }: RgbColor): string {
  const part = (n: number) => clampByte(n).toString(16).padStart(2, '0').toUpperCase();
  return `#${part(r)}${part(g)}${part(b)}`;
}

export function parseHexColor(hex: string): RgbColor | null {
  const cleaned = hex.trim().replace(/^#/, '');
  if (!/^[0-9a-fA-F]{6}$/.test(cleaned)) {
    return null;
  }
  return {
    r: Number.parseInt(cleaned.slice(0, 2), 16),
    g: Number.parseInt(cleaned.slice(2, 4), 16),
    b: Number.parseInt(cleaned.slice(4, 6), 16),
  };
}

export function rgbToHsv({ r, g, b }: RgbColor): HsvColor {
  const rn = clampByte(r) / 255;
  const gn = clampByte(g) / 255;
  const bn = clampByte(b) / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const delta = max - min;

  let h = 0;
  if (delta > 0) {
    if (max === rn) {
      h = 60 * (((gn - bn) / delta) % 6);
    } else if (max === gn) {
      h = 60 * ((bn - rn) / delta + 2);
    } else {
      h = 60 * ((rn - gn) / delta + 4);
    }
  }
  if (h < 0) {
    h += 360;
  }

  const s = max === 0 ? 0 : (delta / max) * 100;
  const v = max * 100;
  return { h, s, v };
}

export function hsvToRgb({ h, s, v }: HsvColor): RgbColor {
  const hue = ((h % 360) + 360) % 360;
  const sat = Math.max(0, Math.min(100, s)) / 100;
  const val = Math.max(0, Math.min(100, v)) / 100;

  const c = val * sat;
  const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = val - c;

  let rn = 0;
  let gn = 0;
  let bn = 0;
  if (hue < 60) {
    rn = c;
    gn = x;
  } else if (hue < 120) {
    rn = x;
    gn = c;
  } else if (hue < 180) {
    gn = c;
    bn = x;
  } else if (hue < 240) {
    gn = x;
    bn = c;
  } else if (hue < 300) {
    rn = x;
    bn = c;
  } else {
    rn = c;
    bn = x;
  }

  return {
    r: clampByte((rn + m) * 255),
    g: clampByte((gn + m) * 255),
    b: clampByte((bn + m) * 255),
  };
}

export interface FriendlyErrorMessage {
  title: string;
  detail: string;
  raw: string;
}

export function formatFriendlyError(rawError: string): FriendlyErrorMessage {
  const lower = rawError.toLowerCase();

  if (
    lower.includes('no se detectó ningún mando') ||
    lower.includes('not connected') ||
    lower.includes('0x0000048f') ||
    lower.includes('no such device') ||
    lower.includes('controllernotfound')
  ) {
    return {
      title: 'Mando no detectado',
      detail: 'Conéctalo por USB o Bluetooth, enciéndelo con el botón PS y pulsa Reintentar.',
      raw: rawError,
    };
  }

  if (
    lower.includes('permisos') ||
    lower.includes('permission') ||
    lower.includes('denied') ||
    lower.includes('access')
  ) {
    return {
      title: 'Acceso bloqueado al mando',
      detail:
        'Otro programa (como Steam Input o un juego) usa el mando en exclusiva, o faltan permisos HID.',
      raw: rawError,
    };
  }

  if (
    lower.includes('writefile') ||
    lower.includes('no respondió') ||
    lower.includes('despertar') ||
    lower.includes('read_timeout') ||
    lower.includes('telemetría')
  ) {
    return {
      title: 'El mando no respondió a tiempo',
      detail: 'Si estás por Bluetooth, pulsa cualquier botón para despertarlo y usa Reintentar.',
      raw: rawError,
    };
  }

  if (lower.includes('hexadecimal') || lower.includes('color no válido') || lower.includes('color inválido')) {
    return {
      title: 'Color no válido',
      detail: 'Introduce un código hexadecimal válido como #00E5FF.',
      raw: rawError,
    };
  }

  return {
    title: 'Problema de conexión con el mando',
    detail:
      rawError.trim().length > 0 && !lower.includes('0x')
        ? rawError
        : 'Comprueba la conexión USB o Bluetooth del DualSense y pulsa Reintentar.',
    raw: rawError,
  };
}
