import { CheckCircle2, ChevronRight, RefreshCw, RotateCcw, ShieldCheck, X } from 'lucide-react';
import type { ConnectionStatus } from '../types/controller';

interface TermsModalProps {
  open: boolean;
  status: ConnectionStatus;
  onClose: () => void;
  onAccept: () => void;
  onResync: () => void;
  onResetDefaults: () => void;
}

export function TermsModal({
  open,
  status,
  onClose,
  onAccept,
  onResync,
  onResetDefaults,
}: TermsModalProps) {
  if (!open) {
    return null;
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/35 p-3 backdrop-blur-md"
      role="presentation"
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="terms-modal-title"
        data-testid="terms-modal"
        className="glass-modal relative flex max-h-[620px] w-full flex-col gap-2.5 overflow-hidden rounded-2xl p-3.5 text-slate-100 shadow-2xl"
      >
        <div
          aria-hidden="true"
          className="pointer-events-none absolute -top-16 left-1/2 h-32 w-56 -translate-x-1/2 rounded-full bg-sky-500/18 blur-2xl"
        />
        <div
          aria-hidden="true"
          className="pointer-events-none absolute -bottom-16 right-6 h-28 w-40 rounded-full bg-indigo-500/16 blur-2xl"
        />

        <div className="relative flex items-center justify-between border-b border-white/[0.10] pb-2">
          <div className="flex items-center gap-2">
            <span className="flex h-6 w-6 items-center justify-center rounded-lg border border-sky-400/35 bg-sky-400/12 shadow-[0_0_12px_rgba(56,189,248,0.28)]">
              <ShieldCheck className="h-3.5 w-3.5 text-sky-300" aria-hidden="true" />
            </span>
            <h2 id="terms-modal-title" className="text-xs font-semibold tracking-tight text-slate-50">
              Términos, Condiciones y Diagnóstico
            </h2>
          </div>
          <button
            type="button"
            aria-label="Cerrar términos y condiciones"
            onClick={onClose}
            className="flex h-6 w-6 items-center justify-center rounded-full border border-white/10 bg-white/[0.04] text-slate-300 transition-all hover:border-white/25 hover:bg-white/15 hover:text-white"
          >
            <X className="h-3.5 w-3.5" aria-hidden="true" />
          </button>
        </div>

        <div className="relative flex-1 space-y-2 overflow-y-auto rounded-xl border border-white/[0.09] bg-white/[0.03] p-2.5 text-[11px] leading-relaxed text-slate-200/90 backdrop-blur-md">
          <p className="font-semibold text-white">SenseHue v2.0.0 — Licencia MIT</p>
          <p>
            <strong>1. Uso bajo licencia libre:</strong> Esta aplicación se ofrece gratuitamente y
            «tal cual» («AS IS») bajo licencia MIT, sin garantías implícitas.
          </p>
          <p>
            <strong>2. Seguridad de hardware HID:</strong> Envía únicamente reportes estándar de
            salida USB (<code className="text-sky-300">0x02</code>) y Bluetooth (
            <code className="text-sky-300">0x31</code> + CRC-32{' '}
            <code className="text-sky-300">0xA2</code>) a mandos PS5 DualSense / Edge sin modificar
            firmware ni flash interna.
          </p>
          <p>
            <strong>3. 100 % offline y sin telemetría:</strong> Todos los recursos HTML/JS/CSS están
            embebidos en el ejecutable. No depende de servidores locales ni externos y guarda tu
            configuración en local de forma atómica.
          </p>
          <p>
            <strong>4. Marcas registradas:</strong> DualSense y PlayStation son marcas de Sony
            Interactive Entertainment Inc. Proyecto independiente sin afiliación oficial.
          </p>
          <div className="flex items-center justify-between gap-2 rounded-xl border border-white/[0.08] bg-slate-950/30 px-2.5 py-1.5 text-[10px] text-slate-300 backdrop-blur-sm">
            <span className="flex items-center gap-1.5">
              <span
                aria-hidden="true"
                className={`h-2 w-2 rounded-full ${
                  status.connected
                    ? 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.9)]'
                    : 'bg-slate-500'
                }`}
              />
              Estado HID actual:
            </span>
            <span className="font-semibold text-slate-100">
              {status.connected
                ? `Conectado (${status.model === 'DualSenseEdge' ? 'Edge' : 'DualSense'} · ${
                    status.transport === 'usb' ? 'USB' : 'Bluetooth'
                  })`
                : 'Sin mando detectado (modo autónomo activo)'}
            </span>
          </div>
        </div>

        <div className="relative grid grid-cols-2 gap-2">
          <button
            type="button"
            data-testid="terms-resync-btn"
            onClick={onResync}
            className="ps-action-tile group flex items-center gap-2 rounded-xl px-2.5 py-2 text-left hover:border-sky-400/45 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-400"
          >
            <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-sky-400/35 bg-sky-400/15 text-sky-300 shadow-[0_0_12px_rgba(56,189,248,0.22)] transition-transform group-hover:scale-105">
              <RefreshCw className="h-3.5 w-3.5 transition-transform duration-300 group-hover:rotate-90" aria-hidden="true" />
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-[11px] font-semibold text-slate-100">
                Re-sincronizar Mando
              </span>
              <span className="block truncate text-[9.5px] text-sky-200/75">
                Desbloqueo BT 0x31 · Aplicar RGB
              </span>
            </span>
          </button>

          <button
            type="button"
            data-testid="terms-reset-btn"
            onClick={onResetDefaults}
            className="ps-action-tile group flex items-center gap-2 rounded-xl px-2.5 py-2 text-left hover:border-rose-400/45 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-400"
          >
            <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-rose-400/35 bg-rose-400/15 text-rose-300 shadow-[0_0_12px_rgba(251,113,133,0.2)] transition-transform group-hover:scale-105">
              <RotateCcw className="h-3.5 w-3.5 transition-transform duration-300 group-hover:-rotate-45" aria-hidden="true" />
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-[11px] font-semibold text-slate-100">
                Restablecer Fábrica
              </span>
              <span className="block truncate text-[9.5px] text-rose-200/75">
                Restaurar perfil original
              </span>
            </span>
          </button>
        </div>

        <button
          type="button"
          data-testid="terms-accept-btn"
          onClick={onAccept}
          className="ps-cta-btn group relative flex w-full items-center justify-between gap-2 overflow-hidden rounded-xl px-3 py-2 text-left text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-300"
        >
          <span className="flex min-w-0 items-center gap-2">
            <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-white/35 bg-white/15 shadow-inner">
              <CheckCircle2 className="h-4 w-4 text-white" aria-hidden="true" />
            </span>
            <span className="min-w-0 truncate text-[11.5px] font-semibold tracking-tight text-white">
              Aceptar términos y activar control HID
            </span>
          </span>
          <span className="flex shrink-0 items-center gap-1.5">
            <span className="rounded-full border border-white/30 bg-white/15 px-2 py-0.5 text-[9px] font-bold tracking-wider text-sky-50 uppercase backdrop-blur-sm">
              BT & USB LISTO
            </span>
            <ChevronRight
              className="h-3.5 w-3.5 text-white/85 transition-transform group-hover:translate-x-0.5"
              aria-hidden="true"
            />
          </span>
        </button>
      </div>
    </div>
  );
}
