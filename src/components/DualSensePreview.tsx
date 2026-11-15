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
      aria-label={`Vista previa vectorial de barra de luz del mando ${hex} al ${clampedBrightness}%`}
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

      {/* 100% Original Vector Controller Illustration (zero third-party logos or photos) */}
      <div className="relative w-full">
        <svg
          viewBox="0 0 900 615"
          role="img"
          aria-label="Ilustración vectorial original de mando inalámbrico compatible"
          className="relative z-10 block h-auto w-full drop-shadow-[0_18px_28px_rgba(0,0,0,0.65)] transition-opacity duration-200"
          style={{ opacity: connected ? 1 : 0.85 }}
        >
          <defs>
            <linearGradient id="sh-shell-light" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor="#F5F7FB" />
              <stop offset="52%" stopColor="#DDE4F0" />
              <stop offset="100%" stopColor="#AEB9CB" />
            </linearGradient>
            <linearGradient id="sh-grip-dark" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor="#232A3B" />
              <stop offset="55%" stopColor="#121622" />
              <stop offset="100%" stopColor="#090C14" />
            </linearGradient>
            <linearGradient id="sh-touchpad" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stopColor="#1B2232" />
              <stop offset="60%" stopColor="#101420" />
              <stop offset="100%" stopColor="#090B13" />
            </linearGradient>
            <radialGradient id="sh-stick-cap" cx="40%" cy="35%" r="65%">
              <stop offset="0%" stopColor="#3B4559" />
              <stop offset="70%" stopColor="#171C28" />
              <stop offset="100%" stopColor="#0B0E16" />
            </radialGradient>
            <linearGradient id="sh-shoulder" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor="#2C3448" />
              <stop offset="100%" stopColor="#111622" />
            </linearGradient>
            <filter id="ds-led-bloom" x="-30%" y="-30%" width="160%" height="160%">
              <feGaussianBlur stdDeviation="5" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>

          {/* Stylized Top Triggers & Bumpers */}
          <path
            d="M 162 82 Q 205 56 266 56 L 274 84 L 154 102 Z"
            fill="url(#sh-shoulder)"
            stroke="rgba(255,255,255,0.14)"
            strokeWidth="2"
          />
          <path
            d="M 738 82 Q 695 56 634 56 L 626 84 L 746 102 Z"
            fill="url(#sh-shoulder)"
            stroke="rgba(255,255,255,0.14)"
            strokeWidth="2"
          />

          {/* Main Ergonomic Outer Shell */}
          <path
            d="M 276 48
               C 320 44, 580 44, 624 48
               C 700 54, 756 96, 782 186
               L 832 396
               C 852 486, 822 560, 756 568
               C 702 574, 668 530, 636 456
               L 584 378
               L 316 378
               L 264 456
               C 232 530, 198 574, 144 568
               C 78 560, 48 486, 68 396
               L 118 186
               C 144 96, 200 54, 276 48 Z"
            fill="url(#sh-shell-light)"
            stroke="rgba(255,255,255,0.68)"
            strokeWidth="2.5"
          />

          {/* Dark Ergonomic Lower Chin & Inner Grips */}
          <path
            d="M 268 342
               C 312 310, 588 310, 632 342
               L 696 472
               C 666 476, 646 446, 620 402
               L 578 368
               L 322 368
               L 280 402
               C 254 446, 234 476, 204 472 Z"
            fill="url(#sh-grip-dark)"
            stroke="rgba(255,255,255,0.10)"
            strokeWidth="1.5"
          />

          {/* Beveled Touchpad Surface */}
          <path
            d="M 282 48 L 618 48 L 605 174 C 601 198, 592 208, 566 208 L 334 208 C 308 208, 299 198, 295 174 Z"
            fill="url(#sh-touchpad)"
            stroke="rgba(255,255,255,0.16)"
            strokeWidth="1.8"
          />
          <path
            d="M 304 62 L 596 62 L 584 142 Q 450 96 314 142 Z"
            fill="rgba(255,255,255,0.035)"
          />

          {/* Left Directional Pad (Clean Geometric Cross, No Trademarked Icons) */}
          <g fill="rgba(226,232,240,0.88)" stroke="rgba(15,23,42,0.35)" strokeWidth="1.5">
            <rect x="193" y="152" width="32" height="38" rx="8" />
            <rect x="193" y="218" width="32" height="38" rx="8" />
            <rect x="155" y="185" width="38" height="32" rx="8" />
            <rect x="225" y="185" width="38" height="32" rx="8" />
            <circle cx="209" cy="201" r="10" fill="rgba(148,163,184,0.35)" stroke="none" />
          </g>

          {/* Right Action Pads (Clean Minimalist Gloss Discs, Zero Registered Symbols) */}
          <g>
            {[
              { cx: 691, cy: 158 },
              { cx: 691, cy: 244 },
              { cx: 646, cy: 201 },
              { cx: 736, cy: 201 },
            ].map((btn, idx) => (
              <g key={idx}>
                <circle
                  cx={btn.cx}
                  cy={btn.cy}
                  r="20"
                  fill="#191F2E"
                  stroke="rgba(255,255,255,0.28)"
                  strokeWidth="2"
                />
                <circle cx={btn.cx} cy={btn.cy} r="7" fill="rgba(226,232,240,0.28)" />
              </g>
            ))}
          </g>

          {/* Share & Options Utility Capsules */}
          <rect
            x="256"
            y="98"
            width="13"
            height="28"
            rx="6.5"
            transform="rotate(-14 262 112)"
            fill="#E2E8F0"
            stroke="rgba(15,23,42,0.3)"
            strokeWidth="1.5"
          />
          <rect
            x="631"
            y="98"
            width="13"
            height="28"
            rx="6.5"
            transform="rotate(14 637 112)"
            fill="#E2E8F0"
            stroke="rgba(15,23,42,0.3)"
            strokeWidth="1.5"
          />

          {/* Twin Analog Thumbsticks */}
          {[332, 568].map((cx) => (
            <g key={cx}>
              <circle
                cx={cx}
                cy={298}
                r="52"
                fill="#0D111A"
                stroke="rgba(255,255,255,0.14)"
                strokeWidth="2"
              />
              <circle
                cx={cx}
                cy={298}
                r="38"
                fill="url(#sh-stick-cap)"
                stroke="rgba(255,255,255,0.22)"
                strokeWidth="1.8"
              />
              <circle
                cx={cx}
                cy={298}
                r="24"
                fill="none"
                stroke="rgba(255,255,255,0.08)"
                strokeWidth="1.5"
              />
            </g>
          ))}

          {/* Center Geometric Home Pill & Mute Switch (Original SenseHue Geometry) */}
          <polygon
            points="450,264 468,274 468,294 450,304 432,294 432,274"
            fill="#1D2434"
            stroke="rgba(255,255,255,0.3)"
            strokeWidth="2"
          />
          <circle cx="450" cy="284" r="5" fill="rgba(56,189,248,0.6)" />
          <rect
            x="433"
            y="330"
            width="34"
            height="12"
            rx="6"
            fill="#232B3D"
            stroke="rgba(255,255,255,0.18)"
            strokeWidth="1.4"
          />

          {/* Dynamic RGB Touchpad Lightbar Channels */}
          <g
            data-testid="lightbar-overlay"
            opacity={isLit ? intensity : 0}
            filter="url(#ds-led-bloom)"
            style={{
              filter: glowFilter,
              transition: 'opacity 120ms ease-out, filter 120ms ease-out',
            }}
          >
            <path
              d="M 278 48 Q 283 125 292 178 Q 297 204 324 210"
              fill="none"
              stroke={edgeRgb}
              strokeWidth="12"
              strokeLinecap="round"
              opacity="0.78"
            />
            <path
              d="M 622 48 Q 617 125 608 178 Q 603 204 576 210"
              fill="none"
              stroke={edgeRgb}
              strokeWidth="12"
              strokeLinecap="round"
              opacity="0.78"
            />
            <path
              d="M 278 48 Q 283 125 292 178 Q 297 204 324 210"
              fill="none"
              stroke={coreRgb}
              strokeWidth="4.4"
              strokeLinecap="round"
            />
            <path
              d="M 622 48 Q 617 125 608 178 Q 603 204 576 210"
              fill="none"
              stroke={coreRgb}
              strokeWidth="4.4"
              strokeLinecap="round"
            />
          </g>

          {/* 5 Centered Player Indicator Micro-LEDs */}
          <g data-testid="player-leds-overlay">
            {PLAYER_LED_POSITIONS.map(({ bit, cx }) => {
              const active = (playerMask & bit) !== 0;
              return (
                <circle
                  key={bit}
                  cx={cx}
                  cy={221}
                  r={active ? 3 : 1.9}
                  data-active={active ? 'true' : 'false'}
                  fill={active ? '#E8F4FF' : 'rgba(255,255,255,0.16)'}
                  opacity={active ? 0.96 : 0.4}
                />
              );
            })}
          </g>

          {/* Microphone Mute Amber LED */}
          {micLed !== 'off' && (
            <rect
              data-testid="mic-led-overlay"
              x={435}
              y={349}
              width={30}
              height={4.8}
              rx={2.4}
              fill="#FF9500"
              className={micLed === 'pulse' ? 'animate-mic-pulse' : undefined}
            />
          )}
        </svg>
      </div>
    </figure>
  );
}
