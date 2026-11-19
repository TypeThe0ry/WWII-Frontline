import { useEffect, useRef, useState } from 'preact/hooks';
import { type GatewayClient, GatewayError, type GuestSession, type JoinTicket } from '../../game/gateway-client';
import { applyFilters, joinState, occupancy, sortServers, type ServerFilters, type ServerInfo, type SortKey } from '../../game/server-list';
import type { Capabilities } from '../../game/boot/capabilities';
import { locale, setLocale, t, type Locale } from '../i18n';
import { uiScreen } from '../screens';
import { boolFilters, readPreferences, sortOptions, writePreferences } from './preferences';

export interface LobbyServices {
  gateway: GatewayClient;
  unlockAudio?(): void;
  launch(ticket: JoinTicket, capabilities: Capabilities, onLeave: () => void): Promise<void>;
}

const REFRESH_MS = 10_000;
const ERROR_LABEL: Record<string, string> = {
  timeout: 'ui.browser.timeout', match_full: 'ui.reject.full', unauthorized: 'ui.reject.ticket', bad_nick: 'ui.browser.badNick',
  rate_limited: 'ui.browser.rateLimited', match_unavailable: 'ui.browser.unavailable', no_such_match: 'ui.browser.unavailable',
  version_mismatch: 'ui.reject.version', server_full: 'ui.reject.full', banned: 'ui.reject.banned', invalid_ticket: 'ui.reject.ticket',
  data_hash_mismatch: 'ui.reject.dataHash',
};
function errorKey(error: unknown): string {
  if (error instanceof GatewayError) return ERROR_LABEL[error.code] ?? 'ui.reject.unknown';
  if (error instanceof Error && 'reason' in error && typeof error.reason === 'string') return ERROR_LABEL[error.reason] ?? 'ui.reject.unknown';
  return 'ui.reject.unknown';
}

export function Menu({ capabilities, services }: { capabilities: Capabilities; services: LobbyServices }) {
  const [screen, setScreen] = useState<'menu' | 'browser' | 'faction'>('menu');
  const [preferred, setPreferred] = useState(readPreferences);
  const [list, setList] = useState<ServerInfo[]>();
  const [refresh, setRefresh] = useState(0);
  const [failed, setFailed] = useState(false);
  const [matchId, setMatchId] = useState<string>();
  const [nickname, setNickname] = useState('');
  const [guest, setGuest] = useState<GuestSession>();
  const inFlight = useRef(false);
  const [joining, setJoining] = useState(false);
  const [joinError, setJoinError] = useState<string>();
  const [launched, setLaunched] = useState(false);
  const selected = list?.find((row) => row.id === matchId);

  useEffect(() => { if (!launched) uiScreen.value = screen; }, [screen, launched]);
  useEffect(() => { writePreferences(preferred); }, [preferred]);
  useEffect(() => {
    if (launched) return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    const load = async () => {
      try {
        const next = await services.gateway.list(controller.signal);
        if (!controller.signal.aborted) { setList(next); setFailed(false); }
      } catch {
        if (!controller.signal.aborted) setFailed(true);
      } finally {
        if (!controller.signal.aborted) timer = setTimeout(() => { void load(); }, REFRESH_MS);
      }
    };
    void load();
    return () => { controller.abort(); clearTimeout(timer); };
  }, [services, refresh, launched]);

  function showFaction(row: ServerInfo) {
    if (!joinState(row).enabled) return;
    setMatchId(row.id);
    setJoinError(undefined);
    setScreen('faction');
  }
  async function join() {
    if (!selected || !joinState(selected).enabled || inFlight.current) return;
    inFlight.current = true;
    setJoining(true);
    setJoinError(undefined);
    try {
      // Guest and ticket are ephemeral, never placed in a URL or persisted.
      const budget = AbortSignal.timeout(5_000);
      const identity = guest ?? await services.gateway.guest(budget);
      setGuest(identity);
      const ticket = await services.gateway.join(selected.id, identity, nickname, budget);
      await services.launch(ticket, capabilities, () => { setScreen('browser'); setLaunched(false); });
      setLaunched(true);
    } catch (err) {
      if (err instanceof GatewayError && err.status === 401) setGuest(undefined);
      setJoinError(err instanceof DOMException && err.name === 'TimeoutError' ? 'ui.browser.timeout' : errorKey(err));
      setRefresh((n) => n + 1);
    } finally { inFlight.current = false; setJoining(false); }
  }

  if (launched) return null;
  const best = sortServers(list ?? [], 'recommended').find((row) => joinState(row).enabled);
  const rows = sortServers(applyFilters(list ?? [], preferred.filters), preferred.sort);
  const filter = <K extends keyof ServerFilters>(key: K, value: ServerFilters[K]) => setPreferred((p) => ({ ...p, filters: { ...p.filters, [key]: value } }));
  const selectFilter = (key: 'map' | 'mode' | 'region') => {
    const current = preferred.filters[key] ?? '';
    const values = [...new Set([...(list ?? []).map((s) => s[key]), ...(current ? [current] : [])])].sort();
    return <label key={key}>{t(`ui.browser.${key}`)} <select value={current} onChange={(e) => filter(key, e.currentTarget.value)}>
      <option value="">{t('ui.browser.any')}</option>
      {values.map((value) => <option key={value}>{value}</option>)}
    </select></label>;
  };
  return <main>
    <h1>{screen === 'menu' ? t('ui.boot.title') : screen === 'browser' ? t('ui.menu.serverBrowser') : t('ui.faction.title')}</h1>
    {screen === 'menu' && <>
      <label>{t('ui.menu.language')} <select value={locale.value} onChange={(e) => setLocale(e.currentTarget.value as Locale)}>
        <option value="en">English</option><option value="zh-Hans">简体中文</option>
      </select></label>
      <button onClick={() => setScreen('browser')}>{t('ui.menu.serverBrowser')}</button>
      <button disabled={!best} onClick={() => { if (best) showFaction(best); }}>{t('ui.menu.quickJoin')}</button>
      <p>{t('ui.boot.recommended', { tier: t(`ui.boot.tier.${capabilities.recommendedTier}`) })}</p>
    </>}
    {screen === 'browser' && <>
      <button onClick={() => setScreen('menu')}>{t('common.back')}</button>
      <button onClick={() => setRefresh((n) => n + 1)}>{t('ui.browser.refresh')}</button>
      <fieldset>
        <legend>{t('ui.browser.filters')}</legend>
        <label>{t('ui.browser.keyword')} <input value={preferred.filters.keyword ?? ''} onInput={(e) => filter('keyword', e.currentTarget.value)} /></label>
        {(['map', 'mode', 'region'] as const).map(selectFilter)}
        {boolFilters.map((key) => <label key={key}>{t(`ui.browser.${key}`)} <input type="checkbox" checked={preferred.filters[key] ?? false} onChange={(e) => filter(key, e.currentTarget.checked)} /></label>)}
        <label>{t('ui.browser.sort')} <select value={preferred.sort} onChange={(e) => { const sort = e.currentTarget.value as SortKey; setPreferred((p) => ({ ...p, sort })); }}>
          {sortOptions.map((key) => <option value={key} key={key}>{t(`ui.browser.sort.${key}`)}</option>)}
        </select></label>
      </fieldset>
      {rows.length > 0 && <table>
        <thead><tr>{['name', 'map', 'mode', 'players', 'region', 'version', 'phase', 'join'].map((key) => <th key={key} scope="col">{t(`ui.browser.${key}`)}</th>)}</tr></thead>
        <tbody>{rows.map((row) => {
          const state = joinState(row);
          return <tr key={row.id} onDblClick={() => showFaction(row)}>
            <th scope="row">{row.name}</th><td>{row.map}</td><td>{row.mode}</td><td>{occupancy(row)}</td><td>{row.region}</td><td>{row.version}</td><td>{row.phase}</td>
            <td><button disabled={!state.enabled} onClick={() => showFaction(row)}>{t('ui.browser.join')}</button>{state.reasonKey && <p>{t(state.reasonKey)}</p>}</td>
          </tr>;
        })}</tbody>
      </table>}
      {list && rows.length === 0 && !failed && <p role="status">{t('ui.browser.empty')}</p>}
    </>}
    {screen === 'faction' && <>
      <button disabled={joining} onClick={() => setScreen('browser')}>{t('common.back')}</button>
      {selected && <h2>{selected.name}</h2>}
      <p>{t('ui.faction.serverAssigns')}</p>
      <form onSubmit={(e) => { e.preventDefault(); void join(); }}>
        <label>{t('ui.browser.nickname')} <input disabled={joining} value={nickname} onInput={(e) => setNickname(e.currentTarget.value)} /></label>
        <button disabled={joining || !selected || !joinState(selected).enabled}>{t(joining ? 'ui.browser.joining' : 'ui.faction.autoJoin')}</button>
      </form>
      {joinError && <p role="alert">{t(joinError)}</p>}
      {!selected && <p role="status">{t('ui.browser.unavailable')}</p>}
      {selected && joinState(selected).reasonKey && <p>{t(joinState(selected).reasonKey!)}</p>}
    </>}
    {!list && !failed && <p role="status">{t('common.loading')}</p>}
    {failed && <p role="alert">{t('ui.menu.serversFailed')} <button onClick={() => setRefresh((n) => n + 1)}>{t('ui.menu.retry')}</button></p>}
  </main>;
}
