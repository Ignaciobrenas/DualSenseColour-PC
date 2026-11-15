import controllerRenderUrl from '../assets/dualsense-render.png';

export type MicLedMode = 'off' | 'on' | 'pulse';

export interface DualSensePreviewProps {
  r: number;
  g: number;
  b: number;
  brightness: number;
  playerLeds?: number;
  micLed?: MicLedMode;
  connected?: boolean;
}

const PLAYER_MASKS: Record<number, number> = {
  0: 0x00,
  1: 0x04, // ..o..
  2: 0x0a, // .o.o.
  3: 0x15, // o.o.o
  4: 0x1b, // oo.oo
  5: 0x1f, // ooooo
};

const PLAYER_LED_POSITIONS = [
  { bit: 0x10, cx: 416 },
  { bit: 0x08, cx: 433 },
  { bit: 0x04, cx: 450 },
  { bit: 0x02, cx: 467 },
  { bit: 0x01, cx: 484 },
];

function clampChannel(value: number): number {
  return Math.max(0, Math.min(255, Math.round(value)));
}

function toHex(r: number, g: number, b: number): string {
  const part = (n: number) => clampChannel(n).toString(16).padStart(2, '0').toUpperCase();
  return `#${part(r)}${part(g)}${part(b)}`;
}

export function DualSensePreview({
  r,
  g,
  b,
  brightness,
  playerLeds = 1,
  micLed = 'off',
  connected = true,
}: DualSensePreviewProps) {
  const cr = clampChannel(r);
  const cg = clampChannel(g);
  const cb = clampChannel(b);
  const clampedBrightness = Math.max(0, Math.min(100, brightness));
  const intensity = clampedBrightness / 100;
  const isLit = intensity > 0 && (cr > 0 || cg > 0 || cb > 0);
  const hex = toHex(cr, cg, cb);

  // Mix slight white core into bright LED strips so center diode looks genuinely luminescent.
  const coreR = Math.round(cr + (255 - cr) * 0.42);
  const coreG = Math.round(cg + (255 - cg) * 0.42);
  const coreB = Math.round(cb + (255 - cb) * 0.42);
  const coreRgb = `rgb(${coreR}, ${coreG}, ${coreB})`;
  const edgeRgb = `rgb(${cr}, ${cg}, ${cb})`;

  const normalizedPlayers = Math.max(0, Math.min(5, Math.round(playerLeds)));
  const playerMask = PLAYER_MASKS[normalizedPlayers] ?? 0x04;

  const glowFilter = isLit
    ? `drop-shadow(0 0 ${Math.round(5 * intensity + 2)}px ${edgeRgb}) drop-shadow(0 0 ${Math.round(14 * intensity + 4)}px rgba(${cr}, ${cg}, ${cb}, ${(0.85 * intensity).toFixed(2)}))`
    : 'none';

  return (
    <figure
      aria-label={`Vista previa de barra de luz del mando ${hex} al ${clampedBrightness}%`}
      data-testid="dualsense-preview"
      data-color={hex}
      data-brightness={clampedBrightness}
      className="relative mx-auto flex w-full max-w-[325px] select-none items-center justify-center py-0.5"
    >
      {/* Ambient backdrop halo diffused behind controller */}
      <div
        data-testid="dualsense-ambient-halo"
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-4 top-2 bottom-4 rounded-full blur-2xl transition-opacity duration-150"
        style={{
          background: isLit
            ? `radial-gradient(circle at 50% 34%, rgba(${cr}, ${cg}, ${cb}, ${(0.42 * intensity).toFixed(2)}) 0%, rgba(${cr}, ${cg}, ${cb}, ${(0.12 * intensity).toFixed(2)}) 52%, transparent 78%)`
            : 'transparent',
          opacity: isLit ? (connected ? 1 : 0.72) : 0,
        }}
      />

      {/* Base hardware render (modified without PlayStation center logo) */}
      <div className="relative w-full">
        <img
          src={controllerRenderUrl}
          alt="Vista previa del mando inalámbrico DualSense"
          draggable={false}
          className="relative z-10 block h-auto w-full drop-shadow-[0_18px_28px_rgba(0,0,0,0.65)] transition-opacity duration-200"
          style={{ opacity: connected ? 1 : 0.88 }}
        />

        {/* Real-time optical LED overlay matching 900x615 controller seam coordinates */}
        <svg
          viewBox="0 0 900 615"
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 z-20 h-full w-full"
          style={{
            mixBlendMode: 'screen',
            filter: glowFilter,
            transition: 'opacity 120ms ease-out, filter 120ms ease-out',
          }}
        >
          <defs>
            <filter id="ds-led-bloom" x="-30%" y="-30%" width="160%" height="160%">
              <feGaussianBlur stdDeviation="5" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>

          {/* Left & Right Touchpad Lightbar channels */}
          <g
            data-testid="lightbar-overlay"
            opacity={isLit ? intensity : 0}
            filter="url(#ds-led-bloom)"
          >
            {/* Outer diffuse colored bloom */}
            <path
              d="M 278 46 Q 283 125 292 178 Q 297 204 324 210"
              fill="none"
              stroke={edgeRgb}
              strokeWidth="12"
              strokeLinecap="round"
              opacity="0.72"
            />
            <path
              d="M 622 46 Q 617 125 608 178 Q 603 204 576 210"
              fill="none"
              stroke={edgeRgb}
              strokeWidth="12"
              strokeLinecap="round"
              opacity="0.72"
            />

            {/* Crisp inner luminescent filament */}
            <path
              d="M 278 46 Q 283 125 292 178 Q 297 204 324 210"
              fill="none"
              stroke={coreRgb}
              strokeWidth="4.2"
              strokeLinecap="round"
            />
            <path
              d="M 622 46 Q 617 125 608 178 Q 603 204 576 210"
              fill="none"
              stroke={coreRgb}
              strokeWidth="4.2"
              strokeLinecap="round"
            />
          </g>

          {/* 5 Player Indicator micro-LEDs centered below touchpad */}
          <g data-testid="player-leds-overlay">
            {PLAYER_LED_POSITIONS.map(({ bit, cx }) => {
              const active = (playerMask & bit) !== 0;
              return (
                <circle
                  key={bit}
                  cx={cx}
                  cy={221}
                  r={active ? 2.6 : 1.8}
                  data-active={active ? 'true' : 'false'}
                  fill={active ? '#E8F4FF' : 'rgba(255,255,255,0.12)'}
                  opacity={active ? 0.96 : 0.35}
                />
              );
            })}
          </g>

          {/* Microphone Mute LED indicator */}
          {micLed !== 'off' && (
            <rect
              data-testid="mic-led-overlay"
              x={435}
              y={349}
              width={30}
              height={4.5}
              rx={2.25}
              fill="#FF9500"
              className={micLed === 'pulse' ? 'animate-mic-pulse' : undefined}
            />
          )}
        </svg>
      </div>
    </figure>
  );
}
