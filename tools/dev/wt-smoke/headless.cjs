// Headless Chromium run of index.html against a local server (M0-017 browser check; the formal
// browser matrix is M0-028). Usage:
//   cargo build --release -p frontline-server
//   node tools/dev/wt-smoke/headless.cjs [out.txt]      # needs `playwright` resolvable (NODE_PATH)
const { chromium } = require('playwright');
const { spawn, execFileSync } = require('node:child_process');
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '../../..');
const bin = (n) => path.join(root, 'target/release', n);
const out = process.argv[2] ?? path.join(root, 'reports/m0/M0-017-chromium-smoke.txt');
(async () => {
  const gen = execFileSync(bin('gen_dev_cert'), ['--out', fs.mkdtempSync('/tmp/wt-smoke-')]).toString();
  const hello = /hello=([0-9a-f]+)/.exec(gen)[1], c1 = /c1=([0-9a-f]+)/.exec(gen)[1];
  const srv = spawn(bin('frontline-server'), ['--dev-cert', '--dev-echo', '--ws-port', '0', '--wt-port', '0', '--metrics-port', '0'], { stdio: ['ignore', 'pipe', 'inherit'] });
  let info, buf = '';
  const srvLines = [];
  await new Promise((res) => srv.stdout.on('data', (d) => { buf += d; for (const l of buf.split('\n')) { try { const v = JSON.parse(l); if (v.event === 'wt_session' && !srvLines.includes(l)) srvLines.push(l); if (v.event === 'wt_listening') { info = v; res(); } } catch {} } }));
  const web = http.createServer((_, r) => r.writeHead(200, { 'content-type': 'text/html' }).end(fs.readFileSync(path.join(__dirname, 'index.html')))).listen(0, '127.0.0.1');
  await new Promise((r) => web.once('listening', r));
  const browser = await chromium.launch();
  const page = await browser.newPage(), logs = [];
  page.on('console', (m) => logs.push('console: ' + m.text()));
  page.on('pageerror', (e) => logs.push('pageerror: ' + e.message));
  const u = `http://localhost:${web.address().port}/?url=${encodeURIComponent(`https://127.0.0.1:${info.port}/wt`)}&certhash=${info.cert_sha256_hex}&hello=${hello}&c1=${c1}`;
  await page.goto(u);
  let ok = true;
  try { await page.waitForFunction(() => document.title.startsWith('done'), null, { timeout: 15000 }); } catch { ok = false; }
  const text = await page.evaluate(() => document.getElementById('o')?.textContent ?? '');
  const ua = await page.evaluate(() => navigator.userAgent);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out, `UA: ${ua}\nserver wt port ${info.port}, cert sha256 ${info.cert_sha256_hex}\n--- page output ---\n${text}\n--- server (debug) ---\n${srvLines.join('\n')}\n--- logs ---\n${logs.join('\n')}\nresult: ${ok ? 'DONE' : 'FAILED'}\n`);
  await page.screenshot({ path: out.replace(/\.txt$/, '.png') });
  await new Promise((r) => setTimeout(r, 300));
  await browser.close(); web.close(); srv.kill('SIGTERM');
  console.log(fs.readFileSync(out, 'utf8'));
  process.exit(ok ? 0 : 1);
})();
