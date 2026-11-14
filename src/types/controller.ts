export type TransportType = 'usb' | 'bluetooth';
export type DeviceModel = 'DualSense' | 'DualSenseEdge';
export type MicLedState = 'off' | 'on' | 'pulse';

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
}

export const BUILTIN_PRESETS: ColorPreset[] = [
  { id: 'playstation-blue', name: 'PlayStation Blue', color: { r: 0, g: 67, b: 156 }, hex: '#00439C' },
  { id: 'crimson-red', name: 'Crimson Red', color: { r: 255, g: 23, b: 68 }, hex: '#FF1744' },
  { id: 'cyber-cyan', name: 'Cyber Cyan', color: { r: 0, g: 240, b: 255 }, hex: '#00F0FF' },
  { id: 'emerald', name: 'Emerald', color: { r: 0, g: 230, b: 118 }, hex: '#00E676' },
  { id: 'gold', name: 'Gold', color: { r: 255, g: 214, b: 0 }, hex: '#FFD600' },
  { id: 'violet', name: 'Violet', color: { r: 157, g: 78, b: 221 }, hex: '#9D4EDD' },
  { id: 'off', name: 'Apagado', color: { r: 0, g: 0, b: 0 }, hex: '#000000' },
];

export const DEFAULT_CONFIG: AppConfig = {
  color: { r: 0, g: 67, b: 156 },
  brightness: 100,
  preset_id: 'playstation-blue',
  player_leds: 1,
  mic_led: 'off',
  apply_on_connect: true,
  keep_alive_steam: false,
  close_to_tray: true,
};

export const DEFAULT_STATUS: ConnectionStatus = {
  connected: false,
  transport: null,
  model: null,
  serial_number: null,
  product_name: null,
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
