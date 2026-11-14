import { CheckCircle2, RefreshCw, RotateCcw, ShieldCheck, X } from 'lucide-react';
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
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-3 backdrop-blur-sm"
      role="presentation"
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="terms-modal-title"
        data-testid="terms-modal"
        className="glass-shell flex max-h-[620px] w-full flex-col gap-2.5 rounded-2xl p-3.5 text-slate-100 shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-white/10 pb-2">
          <div className="flex items-center gap-2">
            <ShieldCheck className="h-4 w-4 text-sky-400" aria-hidden="true" />
            <h2 id="terms-modal-title" className="text-xs font-semibold tracking-tight">
              Términos, Condiciones y Diagnóstico
            </h2>
          </div>
          <button
            type="button"
            aria-label="Cerrar términos y condiciones"
            onClick={onClose}
            className="flex h-6 w-6 items-center justify-center rounded-full text-slate-400 transition-colors hover:bg-white/10 hover:text-slate-100"
          >
            <X className="h-3.5 w-3.5" aria-hidden="true" />
          </button>
        </div>

        <div className="glass-card flex-1 space-y-2 overflow-y-auto rounded-xl p-2.5 text-[11px] leading-relaxed text-slate-300">
          <p className="font-semibold text-slate-100">DualSenseColour PC v1.0.0 — Licencia MIT</p>
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
          <div className="rounded-lg border border-white/10 bg-black/30 px-2.5 py-1.5 text-[10px] text-slate-400">
            Estado HID actual:{' '}
            <span className="font-semibold text-slate-200">
              {status.connected
                ? `Conectado (${status.model === 'DualSenseEdge' ? 'Edge' : 'DualSense'} · ${
                    status.transport === 'usb' ? 'USB' : 'Bluetooth'
                  })`
                : 'Sin mando detectado (modo autónomo activo)'}
            </span>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-1.5">
          <button
            type="button"
            data-testid="terms-resync-btn"
            onClick={onResync}
            className="glass-pill inline-flex items-center justify-center gap-1.5 rounded-xl px-2.5 py-1.5 text-[11px] font-medium text-sky-200 hover:border-sky-400/40"
          >
            <RefreshCw className="h-3 w-3" aria-hidden="true" />
            Sincronizar mando
          </button>
          <button
            type="button"
            data-testid="terms-reset-btn"
            onClick={onResetDefaults}
            className="glass-pill inline-flex items-center justify-center gap-1.5 rounded-xl px-2.5 py-1.5 text-[11px] font-medium text-amber-200 hover:border-amber-400/40"
          >
            <RotateCcw className="h-3 w-3" aria-hidden="true" />
            Valores de fábrica
          </button>
        </div>

        <button
          type="button"
          data-testid="terms-accept-btn"
          onClick={onAccept}
          className="inline-flex w-full items-center justify-center gap-1.5 rounded-xl bg-sky-500/85 px-3 py-2 text-xs font-semibold text-white transition-colors hover:bg-sky-400"
        >
          <CheckCircle2 className="h-3.5 w-3.5" aria-hidden="true" />
          Aceptar términos y continuar
        </button>
      </div>
    </div>
  );
}
