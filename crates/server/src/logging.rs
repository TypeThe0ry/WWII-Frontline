//! Structured JSON logging (ARCHITECTURE §18.4): one JSON object per line with `ts`, `level`,
//! `match_id`, `tick`, optional `player_id`, `event`, plus the event's own fields. A per-call-site
//! rate limiter caps output at N lines per second; suppressed counts are reported once per
//! window as a `log_suppressed` line.

use std::collections::HashMap;
use std::io::{self, BufWriter, Write};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crossbeam_channel::{bounded, Receiver, Sender};
use serde_json::{Map, Value};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::Layer;

/// Current sim tick, published by the sim thread so every log line carries `tick`.
pub static CURRENT_TICK: AtomicU32 = AtomicU32::new(0);

/// Per-key fixed-window limiter. Time is passed in so tests are deterministic.
#[derive(Debug)]
pub struct RateLimiter {
    limit: u32,
    window: Duration,
    state: HashMap<usize, Window>,
}

#[derive(Debug)]
struct Window {
    start: Instant,
    count: u32,
    suppressed: u32,
}

pub enum Verdict {
    Allow,
    /// Allowed, and `n` events were suppressed in the window that just ended.
    AllowAfterSuppressed(u32),
    Drop,
}

impl RateLimiter {
    pub fn new(limit: u32) -> Self {
        RateLimiter {
            limit,
            window: Duration::from_secs(1),
            state: HashMap::new(),
        }
    }

    pub fn check(&mut self, key: usize, now: Instant) -> Verdict {
        let w = self.state.entry(key).or_insert(Window {
            start: now,
            count: 0,
            suppressed: 0,
        });
        let mut report = 0;
        if now.duration_since(w.start) >= self.window {
            report = w.suppressed;
            *w = Window {
                start: now,
                count: 0,
                suppressed: 0,
            };
        }
        if w.count < self.limit {
            w.count += 1;
            if report > 0 {
                Verdict::AllowAfterSuppressed(report)
            } else {
                Verdict::Allow
            }
        } else {
            w.suppressed += 1;
            Verdict::Drop
        }
    }
}

type NowFn = Box<dyn Fn() -> Instant + Send + Sync>;

pub struct JsonLayer<W> {
    match_id: String,
    out: Mutex<W>,
    limiter: Mutex<RateLimiter>,
    now: NowFn,
}

impl<W: Write + Send + 'static> JsonLayer<W> {
    pub fn new(match_id: &str, rate_limit: u32, out: W) -> Self {
        Self::with_clock(match_id, rate_limit, out, Box::new(Instant::now))
    }

    pub fn with_clock(match_id: &str, rate_limit: u32, out: W, now: NowFn) -> Self {
        JsonLayer {
            match_id: match_id.to_string(),
            out: Mutex::new(out),
            limiter: Mutex::new(RateLimiter::new(rate_limit)),
            now,
        }
    }

    fn write_line(&self, level: &str, extra: Map<String, Value>, tick: Option<u64>) {
        let mut obj = Map::new();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        obj.insert("ts".into(), Value::from((ts * 1000.0).round() / 1000.0));
        obj.insert("level".into(), Value::from(level));
        obj.insert("match_id".into(), Value::from(self.match_id.clone()));
        obj.insert(
            "tick".into(),
            Value::from(tick.unwrap_or_else(|| CURRENT_TICK.load(Ordering::Relaxed) as u64)),
        );
        for (k, v) in extra {
            obj.insert(k, v);
        }
        if let Ok(mut line) = serde_json::to_vec(&Value::Object(obj)) {
            line.push(b'\n');
            if let Ok(mut w) = self.out.lock() {
                let _ = w.write_all(&line);
                let _ = w.flush();
            }
        }
    }
}

#[derive(Default)]
struct Fields(Map<String, Value>);

impl Visit for Fields {
    fn record_f64(&mut self, f: &Field, v: f64) {
        self.0.insert(f.name().into(), Value::from(v));
    }
    fn record_i64(&mut self, f: &Field, v: i64) {
        self.0.insert(f.name().into(), Value::from(v));
    }
    fn record_u64(&mut self, f: &Field, v: u64) {
        self.0.insert(f.name().into(), Value::from(v));
    }
    fn record_bool(&mut self, f: &Field, v: bool) {
        self.0.insert(f.name().into(), Value::from(v));
    }
    fn record_str(&mut self, f: &Field, v: &str) {
        self.0.insert(f.name().into(), Value::from(v));
    }
    fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
        self.0
            .insert(f.name().into(), Value::from(format!("{v:?}")));
    }
}

impl<S: Subscriber, W: Write + Send + 'static> Layer<S> for JsonLayer<W> {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let key = event.metadata().callsite().into_usize_for_key();
        let verdict = self
            .limiter
            .lock()
            .map(|mut l| l.check(key, (self.now)()))
            .unwrap_or(Verdict::Allow);
        match verdict {
            Verdict::Drop => return,
            Verdict::AllowAfterSuppressed(n) => {
                let mut m = Map::new();
                m.insert("event".into(), Value::from("log_suppressed"));
                m.insert("suppressed".into(), Value::from(n));
                m.insert("target".into(), Value::from(event.metadata().target()));
                self.write_line("WARN", m, None);
            }
            Verdict::Allow => {}
        }
        let mut f = Fields::default();
        event.record(&mut f);
        let mut m = f.0;
        let tick = m.remove("tick").and_then(|v| v.as_u64());
        if let Some(msg) = m.remove("message") {
            if m.contains_key("event") {
                m.insert("msg".into(), msg);
            } else {
                m.insert("event".into(), msg);
            }
        }
        m.insert("target".into(), Value::from(event.metadata().target()));
        self.write_line(event.metadata().level().as_str(), m, tick);
    }
}

/// `Identifier` has no public integer; hash it to a stable per-call-site key.
trait CallsiteKey {
    fn into_usize_for_key(self) -> usize;
}

impl CallsiteKey for tracing::callsite::Identifier {
    fn into_usize_for_key(self) -> usize {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut h);
        h.finish() as usize
    }
}

const OUTPUT_CAPACITY: usize = 8192;
const FLUSH_INTERVAL: Duration = Duration::from_millis(50);

static AOI_OUTPUT: OnceLock<Output> = OnceLock::new();

enum OutputLine {
    Json(Vec<u8>),
    Aoi(u8, u32, crate::snapshot::priority::Observer),
    Stop,
}

#[derive(Default)]
struct Dropped {
    json: AtomicU64,
    aoi: AtomicU64,
}

#[derive(Clone)]
pub(crate) struct Output {
    tx: Sender<OutputLine>,
    dropped: Arc<Dropped>,
}

impl Output {
    pub(crate) fn aoi(&self, player: u8, tick: u32, observer: crate::snapshot::priority::Observer) {
        if self
            .tx
            .try_send(OutputLine::Aoi(player, tick, observer))
            .is_err()
        {
            self.dropped.aoi.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub(crate) fn aoi_output() -> Option<Output> {
    if std::env::var_os("FL_AOI_TRACE").is_some() {
        AOI_OUTPUT.get().cloned()
    } else {
        None
    }
}

impl Write for Output {
    fn write(&mut self, line: &[u8]) -> io::Result<usize> {
        if self.tx.try_send(OutputLine::Json(line.to_vec())).is_err() {
            self.dropped.json.fetch_add(1, Ordering::Relaxed);
        }
        Ok(line.len())
    }

    // The logging producer never flushes the OS stream.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn write_output(
    rx: Receiver<OutputLine>,
    dropped: &Dropped,
    stdout: impl Write,
    stderr: impl Write,
) {
    let mut stdout = BufWriter::new(stdout);
    let mut stderr = BufWriter::new(stderr);
    let mut write_errors = 0u64;
    let timer = crossbeam_channel::tick(FLUSH_INTERVAL);
    loop {
        crossbeam_channel::select! {
            recv(rx) -> line => {
                let result = match line {
                    Ok(OutputLine::Json(line)) => stdout.write_all(&line),
                    Ok(OutputLine::Aoi(p, tick, o)) => writeln!(
                        stderr, "[aoi-pose] {p} {tick} {} {} {} {} {} {} {} {}",
                        o.team, o.pos.x, o.pos.y, o.pos.z, o.yaw, o.pitch, o.prev_yaw, o.prev_pitch,
                    ),
                    Ok(OutputLine::Stop) | Err(_) => break,
                };
                write_errors += u64::from(result.is_err());
            }
            recv(timer) -> _ => {
                write_errors += u64::from(stdout.flush().is_err()) + u64::from(stderr.flush().is_err());
            }
        }
    }
    write_errors += u64::from(stdout.flush().is_err()) + u64::from(stderr.flush().is_err());
    let json = dropped.json.load(Ordering::Relaxed);
    let aoi = dropped.aoi.load(Ordering::Relaxed);
    if json + aoi + write_errors != 0 {
        let _ = writeln!(
            stderr,
            "[log-output-loss] json_dropped={json} aoi_dropped={aoi} write_errors={write_errors}"
        );
        let _ = stderr.flush();
    }
}

/// Keeps the output worker alive and drains queued lines after the sim and network have stopped.
#[must_use]
pub struct LogGuard {
    tx: Sender<OutputLine>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for LogGuard {
    fn drop(&mut self) {
        let _ = self.tx.send(OutputLine::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Installs the global subscriber. Disk/pipe backpressure only blocks the output worker.
pub fn init(match_id: &str, level: &str, rate_limit: u32) -> Result<LogGuard, String> {
    let lvl: tracing::level_filters::LevelFilter = level
        .parse()
        .map_err(|_| format!("bad log level '{level}'"))?;
    let (tx, rx) = bounded(OUTPUT_CAPACITY);
    let dropped = Arc::new(Dropped::default());
    let output = Output {
        tx: tx.clone(),
        dropped: dropped.clone(),
    };
    let thread = std::thread::Builder::new()
        .name("log-output".into())
        .spawn(move || write_output(rx, &dropped, io::stdout(), io::stderr()))
        .map_err(|e| e.to_string())?;
    let guard = LogGuard {
        tx,
        thread: Some(thread),
    };
    let layer = JsonLayer::new(match_id, rate_limit, output.clone()).with_filter(lvl);
    let sub = tracing_subscriber::registry().with(layer);
    tracing::subscriber::set_global_default(sub).map_err(|e| e.to_string())?;
    let _ = AOI_OUTPUT.set(output);
    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Sink {
        fn lines(&self) -> Vec<Value> {
            let buf = self.0.lock().unwrap().clone();
            String::from_utf8(buf)
                .unwrap()
                .lines()
                .map(|l| serde_json::from_str(l).expect("valid JSON line"))
                .collect()
        }
    }

    #[test]
    fn output_queue_is_bounded_and_drains_both_streams() {
        let (tx, rx) = bounded(2);
        let dropped = Arc::new(Dropped::default());
        let mut output = Output {
            tx,
            dropped: dropped.clone(),
        };
        let observer = crate::snapshot::priority::Observer {
            pos: frontline_sim::Vec3::new(1.0, 2.0, 3.0),
            team: 0,
            yaw: 4.0,
            pitch: 5.0,
            prev_yaw: 6.0,
            prev_pitch: 7.0,
        };
        output.write_all(b"{\"event\":\"first\"}\n").unwrap();
        output.aoi(8, 9, observer);
        // No receiver is polling yet. Both calls must return even when the queue is full.
        output.write_all(b"discarded\n").unwrap();
        output.aoi(8, 10, observer);
        drop(output);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        write_output(rx, &dropped, &mut stdout, &mut stderr);
        assert_eq!(stdout, b"{\"event\":\"first\"}\n");
        assert_eq!(
            stderr,
            b"[aoi-pose] 8 9 0 1 2 3 4 5 6 7\n[log-output-loss] json_dropped=1 aoi_dropped=1 write_errors=0\n",
        );
    }

    fn frozen() -> NowFn {
        let t0 = Instant::now();
        Box::new(move || t0)
    }

    #[test]
    fn lines_are_json_with_required_fields() {
        let sink = Sink::default();
        let layer = JsonLayer::with_clock("m-1", 100, sink.clone(), frozen());
        let sub = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(sub, || {
            tracing::info!(event = "hello", tick = 7u64, player_id = 3u64, "x");
            tracing::warn!("plain message");
        });
        let lines = sink.lines();
        assert_eq!(lines.len(), 2);
        for l in &lines {
            for k in ["match_id", "tick", "level", "ts"] {
                assert!(l.get(k).is_some(), "missing {k} in {l}");
            }
        }
        assert_eq!(lines[0]["event"], "hello");
        assert_eq!(lines[0]["tick"], 7);
        assert_eq!(lines[0]["player_id"], 3);
        assert_eq!(lines[0]["match_id"], "m-1");
        assert_eq!(lines[1]["event"], "plain message");
        assert_eq!(lines[1]["level"], "WARN");
    }

    #[test]
    fn rate_limit_caps_output_per_second() {
        let sink = Sink::default();
        let limit = 20;
        let layer = JsonLayer::with_clock("m", limit, sink.clone(), frozen());
        let sub = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(sub, || {
            for i in 0..10_000u64 {
                tracing::info!(event = "spam", i, "spam");
            }
        });
        assert_eq!(sink.lines().len(), limit as usize);
    }

    #[test]
    fn limiter_resets_after_window_and_reports_suppressed() {
        let mut l = RateLimiter::new(2);
        let t = Instant::now();
        assert!(matches!(l.check(1, t), Verdict::Allow));
        assert!(matches!(l.check(1, t), Verdict::Allow));
        assert!(matches!(l.check(1, t), Verdict::Drop));
        assert!(matches!(l.check(1, t), Verdict::Drop));
        // other key unaffected
        assert!(matches!(l.check(2, t), Verdict::Allow));
        let later = t + Duration::from_millis(1001);
        assert!(matches!(
            l.check(1, later),
            Verdict::AllowAfterSuppressed(2)
        ));
    }
}
