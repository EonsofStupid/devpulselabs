//! The Signal Ledger — durable, queryable record of stamped fabric facts.
//!
//! One concern (signal-ledger plan Phase 2): write the stamped signal stream
//! as hive-partitioned Parquet segments under `.userdata/analytics/`. Files
//! are the truth; any DuckDB file is a disposable cache over them (Phase 3
//! ADR). The UI never touches the filesystem — a bounded channel feeds a
//! writer thread; overflow drops and counts.
//!
//! Off by default. The app enables it with `--ledger`.

use arrow_array::{Array, ArrayRef, RecordBatch, StringArray, UInt64Array};
use arrow_schema::{DataType, Field, Schema};
use dpl_fabric::{
    BuildEvent, CaptureEvent, CoachEvent, Fabric, IntentEvent, JobEvent, ProviderEvent, Stamped,
};
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Schema contract version — additive columns only.
pub const SCHEMA_VERSION: u32 = 1;

/// Channel capacity. Overflow drops; never blocks a frame.
pub const CHANNEL_CAP: usize = 4096;

/// Flush when this many rows are buffered for a domain.
const BATCH_ROWS: usize = 64;

/// Flush when idle this long since the last enqueue observed by the writer.
const FLUSH_IDLE: Duration = Duration::from_millis(500);

/// Roll a new segment after this many rows written into the current one
/// (across flushes of the same open buffer — we roll per flush file; this
/// caps a single flush's size).
const SEGMENT_MAX_ROWS: usize = 10_000;

/// One ledger row — the fixed column contract (schema v1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerRow {
    pub seq: u64,
    pub ms: u64,
    pub wall_ms: u64,
    pub domain: String,
    pub type_name: String,
    pub body: String,
    /// Reserved for §9.5 classification — empty until that layer lands.
    pub class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchemaDoc {
    pub v: u32,
    pub columns: Vec<String>,
}

impl SchemaDoc {
    pub fn current() -> Self {
        Self {
            v: SCHEMA_VERSION,
            columns: [
                "seq", "ms", "wall_ms", "domain", "type", "body", "class", "dropped",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        }
    }
}

/// What `try_record` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordResult {
    Queued,
    Dropped,
}

/// Live ears on every stamped fabric domain + the writer handle.
pub struct LedgerTap {
    capture: dpl_fabric::CaptureReceiver,
    jobs: dpl_fabric::JobReceiver,
    providers: dpl_fabric::ProviderReceiver,
    build: dpl_fabric::BuildReceiver,
    coach: dpl_fabric::CoachReceiver,
    intent: dpl_fabric::IntentReceiver,
    handle: LedgerHandle,
}

impl LedgerTap {
    /// Subscribe at fabric birth — before boot drills publish — and start
    /// the writer thread under `analytics_root`.
    pub fn open(fabric: &Fabric, analytics_root: PathBuf) -> Result<Self, String> {
        let handle = LedgerHandle::spawn(analytics_root)?;
        Ok(Self {
            capture: fabric.subscribe_capture(),
            jobs: fabric.subscribe_jobs(),
            providers: fabric.subscribe_providers(),
            build: fabric.subscribe_build(),
            coach: fabric.subscribe_coach(),
            intent: fabric.subscribe_intent(),
            handle,
        })
    }

    /// Drain every stamped domain into the writer. Lossy under pressure.
    pub fn drain(&mut self) {
        while let Ok(st) = self.capture.try_recv() {
            let _ = self.handle.try_record(row_capture(&st));
        }
        while let Ok(st) = self.jobs.try_recv() {
            let _ = self.handle.try_record(row_job(&st));
        }
        while let Ok(st) = self.providers.try_recv() {
            let _ = self.handle.try_record(row_provider(&st));
        }
        while let Ok(st) = self.build.try_recv() {
            let _ = self.handle.try_record(row_build(&st));
        }
        while let Ok(st) = self.coach.try_recv() {
            let _ = self.handle.try_record(row_coach(&st));
        }
        while let Ok(st) = self.intent.try_recv() {
            let _ = self.handle.try_record(row_intent(&st));
        }
    }

    pub fn dropped(&self) -> u64 {
        self.handle.dropped()
    }

    /// Flush remaining rows and join the writer. Call before
    /// `process::exit` — exit skips Drop.
    pub fn flush_and_join(self) {
        self.handle.flush_and_join();
    }
}

/// Handle onto the writer thread.
pub struct LedgerHandle {
    tx: SyncSender<WriterMsg>,
    dropped: Arc<AtomicU64>,
    join: Option<JoinHandle<()>>,
}

enum WriterMsg {
    Row(LedgerRow),
    Flush,
    Shutdown,
}

impl LedgerHandle {
    pub fn spawn(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(root.join("events"))
            .map_err(|e| format!("ledger: create {}: {e}", root.display()))?;
        let schema_path = root.join("schema.json");
        if !schema_path.exists() {
            let doc = SchemaDoc::current();
            let body = serde_json::to_string_pretty(&doc)
                .map_err(|e| format!("ledger: schema encode: {e}"))?;
            fs::write(&schema_path, body + "\n")
                .map_err(|e| format!("ledger: write {}: {e}", schema_path.display()))?;
        }
        let (tx, rx) = mpsc::sync_channel(CHANNEL_CAP);
        let dropped = Arc::new(AtomicU64::new(0));
        let dropped_writer = Arc::clone(&dropped);
        let join = thread::Builder::new()
            .name("dpl-ledger".into())
            .spawn(move || writer_loop(root, rx, dropped_writer))
            .map_err(|e| format!("ledger: spawn writer: {e}"))?;
        Ok(Self {
            tx,
            dropped,
            join: Some(join),
        })
    }

    pub fn try_record(&self, row: LedgerRow) -> RecordResult {
        match self.tx.try_send(WriterMsg::Row(row)) {
            Ok(()) => RecordResult::Queued,
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                RecordResult::Dropped
            }
        }
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub fn flush_and_join(mut self) {
        let _ = self.tx.send(WriterMsg::Flush);
        let _ = self.tx.send(WriterMsg::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for LedgerHandle {
    fn drop(&mut self) {
        let _ = self.tx.send(WriterMsg::Flush);
        let _ = self.tx.send(WriterMsg::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn writer_loop(root: PathBuf, rx: mpsc::Receiver<WriterMsg>, dropped: Arc<AtomicU64>) {
    let mut buffers: HashMap<String, Vec<LedgerRow>> = HashMap::new();
    let mut last_activity = Instant::now();
    loop {
        let timeout = FLUSH_IDLE.saturating_sub(last_activity.elapsed());
        match rx.recv_timeout(timeout.max(Duration::from_millis(1))) {
            Ok(WriterMsg::Row(row)) => {
                last_activity = Instant::now();
                let domain = row.domain.clone();
                let buf = buffers.entry(domain.clone()).or_default();
                buf.push(row);
                if buf.len() >= BATCH_ROWS {
                    let rows = buffers.remove(&domain).unwrap_or_default();
                    let _ = flush_segment(&root, &domain, &rows, dropped.load(Ordering::Relaxed));
                }
            }
            Ok(WriterMsg::Flush) | Err(RecvTimeoutError::Timeout) => {
                flush_all(&root, &mut buffers, dropped.load(Ordering::Relaxed));
                last_activity = Instant::now();
            }
            Ok(WriterMsg::Shutdown) => {
                flush_all(&root, &mut buffers, dropped.load(Ordering::Relaxed));
                break;
            }
            Err(RecvTimeoutError::Disconnected) => {
                flush_all(&root, &mut buffers, dropped.load(Ordering::Relaxed));
                break;
            }
        }
    }
}

fn flush_all(root: &Path, buffers: &mut HashMap<String, Vec<LedgerRow>>, dropped: u64) {
    let domains: Vec<String> = buffers.keys().cloned().collect();
    for domain in domains {
        if let Some(rows) = buffers.remove(&domain) {
            if !rows.is_empty() {
                let _ = flush_segment(root, &domain, &rows, dropped);
            }
        }
    }
}

fn flush_segment(
    root: &Path,
    domain: &str,
    rows: &[LedgerRow],
    dropped: u64,
) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    let date = utc_date_today();
    let dir = root.join("events").join(format!("date={date}"));
    fs::create_dir_all(&dir).map_err(|e| format!("ledger: mkdir {}: {e}", dir.display()))?;
    let index = next_segment_index(&dir, domain)?;
    let path = dir.join(format!("{domain}-{index:06}.parquet"));

    // Cap a single file.
    let chunk = if rows.len() > SEGMENT_MAX_ROWS {
        &rows[..SEGMENT_MAX_ROWS]
    } else {
        rows
    };
    write_parquet(&path, chunk, dropped)?;
    Ok(())
}

fn next_segment_index(dir: &Path, domain: &str) -> Result<u32, String> {
    let prefix = format!("{domain}-");
    let mut max = 0u32;
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(1),
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(rest) = name.strip_prefix(&prefix) {
            if let Some(num) = rest.strip_suffix(".parquet") {
                if let Ok(n) = num.parse::<u32>() {
                    max = max.max(n);
                }
            }
        }
    }
    Ok(max.saturating_add(1).max(1))
}

fn arrow_schema() -> Schema {
    Schema::new(vec![
        Field::new("seq", DataType::UInt64, false),
        Field::new("ms", DataType::UInt64, false),
        Field::new("wall_ms", DataType::UInt64, false),
        Field::new("domain", DataType::Utf8, false),
        Field::new("type", DataType::Utf8, false),
        Field::new("body", DataType::Utf8, false),
        Field::new("class", DataType::Utf8, false),
        Field::new("dropped", DataType::UInt64, false),
    ])
}

fn write_parquet(path: &Path, rows: &[LedgerRow], dropped: u64) -> Result<(), String> {
    let n = rows.len();
    let seq: Vec<u64> = rows.iter().map(|r| r.seq).collect();
    let ms: Vec<u64> = rows.iter().map(|r| r.ms).collect();
    let wall: Vec<u64> = rows.iter().map(|r| r.wall_ms).collect();
    let domain: Vec<&str> = rows.iter().map(|r| r.domain.as_str()).collect();
    let types: Vec<&str> = rows.iter().map(|r| r.type_name.as_str()).collect();
    let body: Vec<&str> = rows.iter().map(|r| r.body.as_str()).collect();
    let class: Vec<&str> = rows.iter().map(|r| r.class.as_str()).collect();
    let dropped_col = vec![dropped; n];

    let batch = RecordBatch::try_new(
        Arc::new(arrow_schema()),
        vec![
            Arc::new(UInt64Array::from(seq)) as ArrayRef,
            Arc::new(UInt64Array::from(ms)) as ArrayRef,
            Arc::new(UInt64Array::from(wall)) as ArrayRef,
            Arc::new(StringArray::from(domain)) as ArrayRef,
            Arc::new(StringArray::from(types)) as ArrayRef,
            Arc::new(StringArray::from(body)) as ArrayRef,
            Arc::new(StringArray::from(class)) as ArrayRef,
            Arc::new(UInt64Array::from(dropped_col)) as ArrayRef,
        ],
    )
    .map_err(|e| format!("ledger: record batch: {e}"))?;

    let file = File::create(path).map_err(|e| format!("ledger: create {}: {e}", path.display()))?;
    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();
    let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))
        .map_err(|e| format!("ledger: arrow writer: {e}"))?;
    writer
        .write(&batch)
        .map_err(|e| format!("ledger: write: {e}"))?;
    writer.close().map_err(|e| format!("ledger: close: {e}"))?;
    Ok(())
}

/// Read every row from a segment — the unit/proof surface (no DuckDB).
pub fn read_parquet_rows(path: &Path) -> Result<Vec<LedgerRow>, String> {
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
    let file = File::open(path).map_err(|e| format!("ledger: open {}: {e}", path.display()))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("ledger: reader: {e}"))?;
    let mut reader = builder
        .build()
        .map_err(|e| format!("ledger: build reader: {e}"))?;
    let mut out = Vec::new();
    for batch in reader {
        let batch = batch.map_err(|e| format!("ledger: batch: {e}"))?;
        let seq = col_u64(&batch, "seq")?;
        let ms = col_u64(&batch, "ms")?;
        let wall = col_u64(&batch, "wall_ms")?;
        let domain = col_str(&batch, "domain")?;
        let type_name = col_str(&batch, "type")?;
        let body = col_str(&batch, "body")?;
        let class = col_str(&batch, "class")?;
        for i in 0..batch.num_rows() {
            out.push(LedgerRow {
                seq: seq[i],
                ms: ms[i],
                wall_ms: wall[i],
                domain: domain[i].clone(),
                type_name: type_name[i].clone(),
                body: body[i].clone(),
                class: class[i].clone(),
            });
        }
    }
    Ok(out)
}

fn col_u64(batch: &RecordBatch, name: &str) -> Result<Vec<u64>, String> {
    let col = batch
        .column_by_name(name)
        .ok_or_else(|| format!("missing column {name}"))?;
    let arr = col
        .as_any()
        .downcast_ref::<UInt64Array>()
        .ok_or_else(|| format!("column {name} is not u64"))?;
    Ok((0..arr.len()).map(|i| arr.value(i)).collect())
}

fn col_str(batch: &RecordBatch, name: &str) -> Result<Vec<String>, String> {
    let col = batch
        .column_by_name(name)
        .ok_or_else(|| format!("missing column {name}"))?;
    let arr = col
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| format!("column {name} is not utf8"))?;
    Ok((0..arr.len()).map(|i| arr.value(i).to_owned()).collect())
}

fn wall_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn utc_date_today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Civil date from Unix days — good enough for hive partitions; no chrono dep.
    let days = (secs / 86_400) as i64;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Howard Hinnant's civil_from_days — public domain.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn base_row(
    seq: u64,
    ms: u64,
    domain: &str,
    type_name: &str,
    body: serde_json::Value,
) -> LedgerRow {
    LedgerRow {
        seq,
        ms,
        wall_ms: wall_ms_now(),
        domain: domain.into(),
        type_name: type_name.into(),
        body: body.to_string(),
        class: String::new(),
    }
}

fn row_capture(st: &Stamped<CaptureEvent>) -> LedgerRow {
    let (ty, body) = match &st.ev {
        CaptureEvent::Started { mode, target } => (
            "started",
            serde_json::json!({"mode": mode, "target": target}),
        ),
        CaptureEvent::Saved {
            rel_path,
            kind,
            name,
            location,
            source,
        } => (
            "saved",
            serde_json::json!({
                "rel": rel_path, "kind": kind, "name": name,
                "location": location, "source": source
            }),
        ),
        CaptureEvent::Failed { error } => ("failed", serde_json::json!({"error": error})),
        CaptureEvent::InboxChanged => ("inboxChanged", serde_json::json!({})),
        CaptureEvent::PaletteVisibility { visible } => {
            ("paletteVisibility", serde_json::json!({"visible": visible}))
        }
        CaptureEvent::Recording { on, count } => {
            ("recording", serde_json::json!({"on": on, "count": count}))
        }
        CaptureEvent::Skipped { reason } => ("skipped", serde_json::json!({"reason": reason})),
    };
    base_row(st.seq, st.ms, "capture", ty, body)
}

fn row_job(st: &Stamped<JobEvent>) -> LedgerRow {
    let (ty, body) = match &st.ev {
        JobEvent::Queued { job, label } => {
            ("queued", serde_json::json!({"job": job.0, "label": label}))
        }
        JobEvent::Done { job, out } => ("done", serde_json::json!({"job": job.0, "out": out})),
        JobEvent::Failed { job, error } => {
            ("failed", serde_json::json!({"job": job.0, "error": error}))
        }
    };
    base_row(st.seq, st.ms, "jobs", ty, body)
}

fn row_provider(st: &Stamped<ProviderEvent>) -> LedgerRow {
    // Mirror the seam's redaction posture: ids and stages only, never secrets.
    use ProviderEvent as E;
    let (ty, body) = match &st.ev {
        E::Selected { provider } => ("selected", serde_json::json!({"provider": provider.0})),
        E::ProfileConfigured {
            provider,
            profile_key,
        } => (
            "profileConfigured",
            serde_json::json!({"provider": provider.0, "profileKey": profile_key}),
        ),
        E::ActivationRequested { provider, attempt } => (
            "activationRequested",
            serde_json::json!({"provider": provider.0, "attempt": attempt.0}),
        ),
        E::ClientStarted {
            provider,
            attempt,
            client_version,
        } => (
            "clientStarted",
            serde_json::json!({
                "provider": provider.0, "attempt": attempt.0,
                "clientVersion": client_version
            }),
        ),
        E::DeviceCodeReceived {
            provider,
            attempt,
            code_len,
        } => (
            "deviceCodeReceived",
            serde_json::json!({
                "provider": provider.0, "attempt": attempt.0, "codeLen": code_len
            }),
        ),
        E::CodeCopied { provider, attempt } => (
            "codeCopied",
            serde_json::json!({"provider": provider.0, "attempt": attempt.0}),
        ),
        E::SystemBrowserDispatched { provider, attempt } => (
            "systemBrowserDispatched",
            serde_json::json!({"provider": provider.0, "attempt": attempt.0}),
        ),
        E::AuthorizationCompleted { provider, attempt } => (
            "authorizationCompleted",
            serde_json::json!({"provider": provider.0, "attempt": attempt.0}),
        ),
        E::RuntimeStarted { provider, attempt } => (
            "runtimeStarted",
            serde_json::json!({"provider": provider.0, "attempt": attempt.0}),
        ),
        E::VerificationRetryScheduled {
            provider,
            attempt,
            retry_index,
            max_retries,
            delay_ms,
        } => (
            "verificationRetryScheduled",
            serde_json::json!({
                "provider": provider.0, "attempt": attempt.0,
                "retryIndex": retry_index, "maxRetries": max_retries, "delayMs": delay_ms
            }),
        ),
        E::Ready {
            provider,
            attempt,
            client_version,
            auth_method,
        } => (
            "ready",
            serde_json::json!({
                "provider": provider.0, "attempt": attempt.0,
                "clientVersion": client_version, "authMethod": auth_method
            }),
        ),
        E::Failed {
            provider,
            attempt,
            stage,
            class,
            retryable,
        } => (
            "failed",
            serde_json::json!({
                "provider": provider.0, "attempt": attempt.0,
                "stage": stage.label(), "class": class.label(), "retryable": retryable
            }),
        ),
        E::Cancelled { provider, attempt } => (
            "cancelled",
            serde_json::json!({"provider": provider.0, "attempt": attempt.0}),
        ),
        E::Expired { provider, attempt } => (
            "expired",
            serde_json::json!({"provider": provider.0, "attempt": attempt.0}),
        ),
    };
    base_row(st.seq, st.ms, "providers", ty, body)
}

fn row_build(st: &Stamped<BuildEvent>) -> LedgerRow {
    let (ty, body) = match &st.ev {
        BuildEvent::SlidePlaced {
            show,
            episode,
            chapter,
            slide,
        } => (
            "slidePlaced",
            serde_json::json!({
                "show": show, "episode": episode, "chapter": chapter, "slide": slide
            }),
        ),
        BuildEvent::EpisodeChanged { show, episode } => (
            "episodeChanged",
            serde_json::json!({"show": show, "episode": episode}),
        ),
    };
    base_row(st.seq, st.ms, "build", ty, body)
}

fn row_coach(st: &Stamped<CoachEvent>) -> LedgerRow {
    let (ty, body) = match &st.ev {
        CoachEvent::Armed { coach_id } => ("armed", serde_json::json!({"coachId": coach_id})),
        CoachEvent::Mounted { coach_id } => ("mounted", serde_json::json!({"coachId": coach_id})),
        CoachEvent::Triggered {
            coach_id,
            trigger,
            actor,
        } => (
            "triggered",
            serde_json::json!({
                "coachId": coach_id,
                "trigger": trigger,
                "actor": match actor {
                    dpl_fabric::CoachActor::Operator => "operator",
                    dpl_fabric::CoachActor::Agent => "agent",
                    dpl_fabric::CoachActor::App => "app",
                },
            }),
        ),
        CoachEvent::Satisfied { coach_id } => {
            ("satisfied", serde_json::json!({"coachId": coach_id}))
        }
        CoachEvent::Unmounted { coach_id, reason } => (
            "unmounted",
            serde_json::json!({"coachId": coach_id, "reason": reason}),
        ),
    };
    base_row(st.seq, st.ms, "coach", ty, body)
}

fn row_intent(st: &Stamped<IntentEvent>) -> LedgerRow {
    let (ty, body) = match &st.ev {
        IntentEvent::DeterminismChanged { mode, cause } => {
            let mode = match mode {
                dpl_intent::DeterminismMode::Behavioral => "behavioral",
                dpl_intent::DeterminismMode::Strict => "strict",
            };
            let cause = cause.map(|c| match c {
                dpl_intent::StrictCause::Operator(dpl_intent::IntentChannel::UiAcknowledged) => {
                    "ui-acknowledged"
                }
                dpl_intent::StrictCause::Operator(dpl_intent::IntentChannel::SlashCommand) => {
                    "slash-command"
                }
                dpl_intent::StrictCause::Evidence => "evidence",
            });
            (
                "determinismChanged",
                serde_json::json!({"mode": mode, "cause": cause}),
            )
        }
        IntentEvent::IntentRefused { action, reason } => (
            "intentRefused",
            serde_json::json!({"action": action, "reason": reason}),
        ),
    };
    base_row(st.seq, st.ms, "intent", ty, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_doc_round_trips() {
        let doc = SchemaDoc::current();
        let json = serde_json::to_string(&doc).unwrap();
        let back: SchemaDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(back.v, SCHEMA_VERSION);
        assert!(back.columns.iter().any(|c| c == "seq"));
        assert!(back.columns.iter().any(|c| c == "class"));
    }

    #[test]
    fn parquet_round_trip_preserves_rows() {
        let dir = std::env::temp_dir().join(format!("dpl-ledger-rt-{}", wall_ms_now()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.parquet");
        let rows = vec![
            LedgerRow {
                seq: 1,
                ms: 10,
                wall_ms: 1000,
                domain: "intent".into(),
                type_name: "determinismChanged".into(),
                body: r#"{"mode":"strict","cause":"evidence"}"#.into(),
                class: String::new(),
            },
            LedgerRow {
                seq: 2,
                ms: 20,
                wall_ms: 1001,
                domain: "coach".into(),
                type_name: "satisfied".into(),
                body: r#"{"coachId":"x"}"#.into(),
                class: String::new(),
            },
        ];
        write_parquet(&path, &rows, 0).unwrap();
        let back = read_parquet_rows(&path).unwrap();
        assert_eq!(back, rows);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn overflow_increments_dropped_instead_of_blocking() {
        let dir = std::env::temp_dir().join(format!("dpl-ledger-drop-{}", wall_ms_now()));
        let _ = fs::remove_dir_all(&dir);
        let handle = LedgerHandle::spawn(dir.clone()).unwrap();
        // Fill past capacity with a stuck consumer: we don't flush, we just
        // hammer try_record. The channel is 4096; send more.
        let mut dropped = 0u64;
        for i in 0..(CHANNEL_CAP as u64 + 200) {
            let result = handle.try_record(LedgerRow {
                seq: i,
                ms: i,
                wall_ms: i,
                domain: "capture".into(),
                type_name: "inboxChanged".into(),
                body: "{}".into(),
                class: String::new(),
            });
            if result == RecordResult::Dropped {
                dropped += 1;
            }
        }
        assert!(
            dropped > 0 || handle.dropped() > 0,
            "expected overflow under burst; dropped={dropped} handle={}",
            handle.dropped()
        );
        handle.flush_and_join();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn live_fabric_facts_land_in_a_segment() {
        let dir = std::env::temp_dir().join(format!("dpl-ledger-live-{}", wall_ms_now()));
        let _ = fs::remove_dir_all(&dir);
        let fabric = Fabric::new();
        let mut tap = LedgerTap::open(&fabric, dir.clone()).unwrap();
        fabric.publish_intent(IntentEvent::DeterminismChanged {
            mode: dpl_intent::DeterminismMode::Strict,
            cause: Some(dpl_intent::StrictCause::Evidence),
        });
        fabric.publish_coach(CoachEvent::Satisfied {
            coach_id: "lesson-1/step-2".into(),
        });
        tap.drain();
        // Give the writer a moment, then flush.
        thread::sleep(Duration::from_millis(50));
        tap.flush_and_join();

        let date = utc_date_today();
        let intent_dir = dir.join("events").join(format!("date={date}"));
        let mut found_intent = false;
        let mut found_coach = false;
        if let Ok(entries) = fs::read_dir(&intent_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("parquet") {
                    continue;
                }
                let rows = read_parquet_rows(&path).unwrap();
                for row in rows {
                    if row.domain == "intent" && row.type_name == "determinismChanged" {
                        found_intent = true;
                        assert!(row.body.contains("evidence"));
                    }
                    if row.domain == "coach" && row.type_name == "satisfied" {
                        found_coach = true;
                    }
                }
            }
        }
        assert!(
            found_intent,
            "intent evidence row missing under {intent_dir:?}"
        );
        assert!(
            found_coach,
            "coach satisfied row missing under {intent_dir:?}"
        );
        let schema: SchemaDoc =
            serde_json::from_str(&fs::read_to_string(dir.join("schema.json")).unwrap()).unwrap();
        assert_eq!(schema.v, SCHEMA_VERSION);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn civil_from_days_matches_known_anchors() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        // 2000-01-01 = 10957 days after the Unix epoch.
        assert_eq!(civil_from_days(10_957), (2000, 1, 1));
        // 2024-02-29 (leap day).
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
    }
}
