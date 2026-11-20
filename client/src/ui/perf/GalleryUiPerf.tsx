import { useEffect, useRef, useState } from 'preact/hooks';
import { benchmarkSignals, type SignalResult } from './signal-benchmark';

export function GalleryUiPerf() {
  return import.meta.env.DEV && new URLSearchParams(location.search).get('uiperf') === '1' ? <Probe /> : null;
}

function Probe() {
  const ref = useRef<HTMLOutputElement>(null);
  const [result, setResult] = useState<SignalResult>();
  const [error, setError] = useState('');
  useEffect(() => {
    const root = ref.current?.closest('[data-ui-gallery]');
    if (!root) { setError('Missing data-ui-gallery root'); return; }
    return benchmarkSignals(root, setResult, setError);
  }, []);
  return <output ref={ref} data-uiperf={error ? 'error' : result ? 'done' : 'running'}
    data-mean-ms={result?.meanMs} data-p95-ms={result?.p95Ms} data-frames={result?.frames}
    data-signals={result?.signals} data-warmup={result?.warmup} data-nodes={result?.nodes}>
    {error || (result ? `100 signals: ${result.meanMs.toFixed(3)} ms (mean), ${result.p95Ms.toFixed(3)} ms (P95); DOM: ${result.nodes}` : '100 signals: ...')}
  </output>;
}
