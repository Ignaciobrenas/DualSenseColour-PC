import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Check, Compass, RefreshCw, Search, Sparkles } from 'lucide-react';
import {
  CURATED_COMMUNITY_PRESETS,
  type CommunityProfile,
  type DiscoverShowcaseItem,
  type RegistryIndex,
} from '../types/controller';

const REGISTRY_INDEX_URL =
  'https://raw.githubusercontent.com/Ignaciobrenas/sensehue-presets/main/index.json';

interface DiscoverPanelProps {
  onInstallProfile: (profile: CommunityProfile | string) => Promise<void>;
}

export function DiscoverPanel({ onInstallProfile }: DiscoverPanelProps) {
  const [query, setQuery] = useState('');
  const [items, setItems] = useState<DiscoverShowcaseItem[]>(CURATED_COMMUNITY_PRESETS);
  const [appliedId, setAppliedId] = useState<string | null>(null);
  const [statusMessage, setStatusMessage] = useState<string>(
    'Catálogo curado offline listo · 1 clic para aplicar',
  );
  const [loading, setLoading] = useState(false);
  const timerRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        window.clearTimeout(timerRef.current);
      }
    };
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) {
      return items;
    }
    return items.filter(
      (item) =>
        item.profile.name.toLowerCase().includes(q) ||
        item.profile.author.toLowerCase().includes(q) ||
        item.description.toLowerCase().includes(q),
    );
  }, [items, query]);

  const handleRefreshOnline = async () => {
    setLoading(true);
    try {
      const response = await fetch(REGISTRY_INDEX_URL);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const rawIndex = await response.text();
      const validated = await invoke<RegistryIndex>('validate_community_index', {
        rawJson: rawIndex,
      });

      const resolved: DiscoverShowcaseItem[] = [];
      for (const entry of validated.presets.slice(0, 12)) {
        const res = await fetch(entry.url);
        if (!res.ok) continue;
        const profileRaw = await res.text();
        const parsed = JSON.parse(profileRaw) as CommunityProfile;
        resolved.push({
          id: entry.id,
          description: entry.description,
          profile: parsed,
        });
      }

      if (resolved.length > 0) {
        setItems(resolved);
        setStatusMessage(`${resolved.length} perfiles validados desde GitHub`);
      } else {
        setStatusMessage('Índice sin entradas válidas; manteniendo catálogo curado');
      }
    } catch {
      setItems(CURATED_COMMUNITY_PRESETS);
      setStatusMessage('Modo offline activo · catálogo curado local');
    } finally {
      setLoading(false);
    }
  };

  const handleApply = async (item: DiscoverShowcaseItem) => {
    await onInstallProfile(item.profile);
    setAppliedId(item.id);
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
    }
    timerRef.current = window.setTimeout(() => setAppliedId(null), 1600);
  };

  return (
    <section
      aria-label="Perfiles Comunitarios Discover"
      data-testid="discover-panel"
      className="panel-enter flex w-full min-w-0 flex-col gap-2"
    >
      <div className="glass-card flex items-center gap-1.5 rounded-2xl p-2">
        <Search className="h-3.5 w-3.5 shrink-0 text-slate-400" aria-hidden="true" />
        <input
          type="text"
          data-testid="discover-search-input"
          aria-label="Buscar perfiles comunitarios"
          placeholder="Buscar preset .sensehue o autor..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="min-w-0 flex-1 bg-transparent text-[11px] text-slate-100 placeholder:text-slate-400 focus:outline-none"
        />
        <button
          type="button"
          data-testid="discover-refresh-btn"
          disabled={loading}
          onClick={() => void handleRefreshOnline()}
          className="ps-action-tile inline-flex shrink-0 items-center gap-1 rounded-lg px-2 py-1 text-[10px] font-medium text-slate-200 disabled:opacity-50"
        >
          <RefreshCw className={`h-3 w-3 ${loading ? 'animate-spin' : ''}`} aria-hidden="true" />
          Sync
        </button>
      </div>

      <div className="flex items-center justify-between px-1 text-[9.5px] text-slate-400">
        <span className="inline-flex items-center gap-1">
          <Compass className="h-3 w-3 text-cyan-400" aria-hidden="true" />
          {statusMessage}
        </span>
        <span className="font-mono-tabular">{filtered.length} perfiles</span>
      </div>

      <div className="flex max-h-[215px] flex-col gap-1.5 overflow-y-auto pr-0.5">
        {filtered.map((item) => {
          const isApplied = appliedId === item.id;
          return (
            <div
              key={item.id}
              className="glass-card flex items-center justify-between gap-2 rounded-xl px-2.5 py-2"
            >
              <div className="flex min-w-0 items-center gap-2">
                <span
                  aria-hidden="true"
                  className="h-6 w-6 shrink-0 rounded-full border border-white/25"
                  style={{
                    backgroundColor: item.profile.color,
                    boxShadow: `0 0 12px ${item.profile.color}88`,
                  }}
                />
                <div className="min-w-0">
                  <div className="flex items-center gap-1.5">
                    <p className="truncate text-[11px] font-semibold text-slate-100">
                      {item.profile.name}
                    </p>
                    <span className="font-mono-tabular shrink-0 rounded bg-white/10 px-1 py-0.2 text-[8.5px] text-slate-300">
                      @{item.profile.author}
                    </span>
                  </div>
                  <p className="truncate text-[9.5px] text-slate-300">{item.description}</p>
                  <p className="font-mono-tabular text-[8.5px] text-slate-400">
                    {item.profile.color} · {item.profile.brightness}% · L2:{item.profile.left_trigger}{' '}
                    · R2:{item.profile.right_trigger}
                  </p>
                </div>
              </div>

              <button
                type="button"
                data-testid={`discover-apply-${item.id}`}
                onClick={() => void handleApply(item)}
                className={`ps-action-tile inline-flex shrink-0 items-center gap-1 rounded-lg px-2.5 py-1.5 text-[10px] font-semibold ${
                  isApplied
                    ? 'border-emerald-400/55 bg-emerald-500/20 text-emerald-200'
                    : 'text-sky-100'
                }`}
              >
                {isApplied ? (
                  <>
                    <Check className="h-3 w-3" aria-hidden="true" />
                    Aplicado
                  </>
                ) : (
                  <>
                    <Sparkles className="h-3 w-3 text-cyan-300" aria-hidden="true" />
                    Aplicar
                  </>
                )}
              </button>
            </div>
          );
        })}
      </div>
    </section>
  );
}
