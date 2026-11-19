import { render } from 'preact';
import { useEffect, useState } from 'preact/hooks';
import { backoffMs, bootNext, type BootPhase } from '../../game/boot/boot-fsm';
import { DEGRADED_FEATURES, probeCapabilities, type Capabilities } from '../../game/boot/capabilities';
import { locale, t } from '../i18n';
import '../i18n/catalogs';
import type { LobbyServices } from './menu';

const MAX_RETRIES = 4;
const loadShell = () => import('./menu');

function Boot({ services }: { services: LobbyServices }) {
  const [phase, setPhase] = useState<BootPhase>('probing');
  const [caps, setCaps] = useState<Capabilities>();
  const [attempt, setAttempt] = useState(0);
  const [shell, setShell] = useState<Awaited<ReturnType<typeof loadShell>>>();
  const [entered, setEntered] = useState(false);
  const [copied, setCopied] = useState(false);
  const [copyFailed, setCopyFailed] = useState(false);
  const language = locale.value;

  useEffect(() => {
    let live = true;
    void probeCapabilities(navigator, () => {
      try {
        const gl = document.createElement('canvas').getContext('webgl2');
        gl?.getExtension('WEBGL_lose_context')?.loseContext();
        return !!gl;
      } catch { return false; }
    }).then((result) => {
      if (!live) return;
      setCaps(result);
      setPhase((p) => bootNext(p, { t: 'probed', backend: result.backend }));
    });
    return () => { live = false; };
  }, []);

  useEffect(() => {
    if (phase === 'failed' && attempt < MAX_RETRIES) {
      const timer = setTimeout(() => {
        setAttempt(attempt + 1);
        setPhase((p) => bootNext(p, { t: 'retry' }));
      }, backoffMs(attempt));
      return () => clearTimeout(timer);
    }
    if (phase !== 'shell_loading') return;
    let live = true;
    void loadShell().then((module) => {
      if (!live) return;
      setShell(module);
      setPhase((p) => bootNext(p, { t: 'shell_ok' }));
    }, () => { if (live) setPhase((p) => bootNext(p, { t: 'shell_fail' })); });
    return () => { live = false; };
  }, [phase, attempt]);

  useEffect(() => { document.documentElement.lang = language; }, [language]);
  if (entered && shell && caps) return <shell.Menu capabilities={caps} services={services} />;

  const showDiagnostics = phase === 'unsupported' || phase === 'ready';
  const copyDiagnostics = async () => {
    try {
      await navigator.clipboard.writeText(JSON.stringify(caps, null, 2));
      setCopied(true);
      setCopyFailed(false);
    } catch { setCopyFailed(true); }
  };
  return <main>
    <h1>{t('ui.boot.title')}</h1>
    {phase === 'probing' && <p role="status">{t('ui.boot.probing')}</p>}
    {phase === 'shell_loading' && <p role="status">{t('ui.boot.loading')}</p>}
    {phase === 'failed' && <>
      <p role="alert">{attempt < MAX_RETRIES ? t('ui.boot.failed', { resource: 'UI', seconds: backoffMs(attempt) / 1000 }) : t('ui.boot.retryExhausted')}</p>
      <button onClick={() => { setAttempt(0); setPhase((p) => bootNext(p, { t: 'retry' })); }}>{t('ui.menu.retry')}</button>
    </>}
    {phase === 'unsupported' && <p role="alert">{t('ui.boot.unsupported')}</p>}
    {phase === 'ready' && <>
      {caps?.backend === 'webgl2' && <section>
        <h2>{t('ui.boot.degradedTitle')}</h2>
        <ul>{DEGRADED_FEATURES.map((key) => <li key={key}>{t(key)}</li>)}</ul>
      </section>}
      <button onClick={() => { services.unlockAudio?.(); setEntered(true); }}>{t('ui.boot.clickToContinue')}</button>
    </>}
    {showDiagnostics && <>
      <button onClick={() => { void copyDiagnostics(); }}>{t('ui.boot.copyDiag')}</button>
      {copied && <p role="status">{t('ui.boot.copied')}</p>}
      {copyFailed && <pre>{JSON.stringify(caps, null, 2)}</pre>}
    </>}
  </main>;
}

export function mountLobby(element: HTMLElement, services: LobbyServices): () => void {
  render(<Boot services={services} />, element);
  return () => render(null, element);
}
