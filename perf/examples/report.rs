//! Comparison tables from the last benchmark runs, as markdown: criterion's medians pivoted by
//! parameter with the best of each column in bold, the spin counts and gaps beside them, and the
//! latency percentiles.
//!
//! ```sh
//! cargo run -p perf --example report -- [--baseline <name>] [--out <file>] [<filter>]
//! ```
//!
//! `--baseline` adds each cell's change against a criterion baseline saved with
//! `--save-baseline <name>`, positive when faster. A filter keeps the groups whose name contains it.

#[path = "../benches/harness/mod.rs"]
mod harness;

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, thread};

use harness::{latency, output};
use serde_json::Value;

/// Criterion groups in report order, with what each measures.
const GROUPS: [(&str, &str); 11] = [
  ("push_pop", "push one record and pop it back, one thread"),
  ("push_full", "push into a full queue, one thread: the price of a refusal"),
  ("pop_empty", "pop from an empty queue, one thread: the price of one look"),
  (
    "fill_drain",
    "fill to capacity, then drain, one thread; columns are capacities",
  ),
  ("spsc", "one producer thread, one consumer thread; columns are capacities"),
  (
    "spsc_payload",
    "one producer, one consumer, 1024 slots; columns are record sizes in bytes",
  ),
  (
    "spsc_pinned",
    "one producer, one consumer, 1024 slots, on SMT siblings of one core and on two cores",
  ),
  ("batch", "one producer pushing that many records per operation, 1024 slots"),
  ("mpsc_producers", "records split over that many producers, 1024 slots"),
  ("mpsc_capacity", "four producers; columns are capacities"),
  ("mpsc_oversubscribed", "twice as many producers as logical CPUs, 1024 slots"),
];

/// Row order: the in-house rings first, then the references.
const CANDIDATES: [&str; 4] = ["spsc", "mpsc", "rtrb", "mutex"];

struct Args {
  baseline: Option<String>,
  out: Option<PathBuf>,
  filter: Option<String>,
}

impl Args {
  fn parse() -> Self {
    let mut args = Self {
      baseline: None,
      out: None,
      filter: None,
    };
    let mut words = env::args().skip(1);
    while let Some(word) = words.next() {
      match word.as_str() {
        "--baseline" => args.baseline = words.next(),
        "--out" => args.out = words.next().map(PathBuf::from),
        "--help" | "-h" => {
          println!("report [--baseline <name>] [--out <file>] [<filter>]");
          std::process::exit(0);
        }
        flag if flag.starts_with('-') => {
          eprintln!("report: unknown flag {flag}");
          std::process::exit(2);
        }
        _ => args.filter = Some(word),
      }
    }

    args
  }

  fn wants(&self, group: &str) -> bool {
    self.filter.as_deref().is_none_or(|filter| group.contains(filter))
  }
}

/// One benchmark's result.
struct Entry {
  function: String,
  value: Option<String>,
  /// Records per iteration.
  elements: u64,
  /// Median nanoseconds per iteration.
  median: f64,
  /// The same, in the baseline.
  baseline: Option<f64>,
}

impl Entry {
  fn id(&self, group: &str) -> String {
    match &self.value {
      Some(value) => format!("{group}/{}/{value}", self.function),
      None => format!("{group}/{}", self.function),
    }
  }
}

fn main() {
  let args = Args::parse();
  let criterion = output::criterion_dir();
  let perf = output::perf_dir();

  let mut report = format!("# Benchmark comparison\n\n{}\n\n", machine());
  if let Some(name) = &args.baseline {
    let _ = writeln!(
      report,
      "Cells carry their change against the baseline `{name}`, positive when faster.\n"
    );
  }

  let groups = entries(&criterion, args.baseline.as_deref());
  if groups.is_empty() {
    let _ = writeln!(report, "No criterion results under `{}`.\n", criterion.display());
  }
  let extras = extras(&perf);
  let known: Vec<&str> = GROUPS.iter().map(|(name, _)| *name).collect();
  let others = groups
    .keys()
    .filter(|name| !known.contains(&name.as_str()))
    .map(|name| (name.as_str(), ""));
  for (name, about) in GROUPS.into_iter().chain(others) {
    if let Some(entries) = groups.get(name).filter(|_| args.wants(name)) {
      table(&mut report, name, about, entries, &extras);
    }
  }

  if args.wants("latency") {
    let rows: Vec<Value> = fs::read_to_string(output::latency_file(&perf))
      .unwrap_or_default()
      .lines()
      .filter_map(|line| serde_json::from_str(line).ok())
      .collect();
    report.push_str(&latency::markdown(&rows));
  }

  match &args.out {
    Some(path) => output::save(path, &report),
    None => print!("{report}"),
  }
}

/// Every group's entries, from criterion's `new/benchmark.json` and `new/estimates.json` files.
fn entries(criterion: &Path, baseline: Option<&str>) -> BTreeMap<String, Vec<Entry>> {
  let mut files = Vec::new();
  find(criterion, &mut files);
  let mut groups: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
  for file in files {
    let Some(dir) = file.parent().and_then(Path::parent) else {
      continue;
    };
    let (Some(benchmark), Some(new)) = (json(&file), median(&dir.join("new"))) else {
      continue;
    };
    let text = |field: &str| benchmark[field].as_str().map(str::to_string);
    let Some(group) = text("group_id") else { continue };
    groups.entry(group).or_default().push(Entry {
      function: text("function_id").unwrap_or_default(),
      value: text("value_str"),
      elements: benchmark["throughput"]["Elements"].as_u64().unwrap_or(1),
      median: new,
      baseline: baseline.and_then(|name| median(&dir.join(name))),
    });
  }

  groups
}

fn find(dir: &Path, files: &mut Vec<PathBuf>) {
  let Ok(read) = fs::read_dir(dir) else { return };
  for entry in read.flatten() {
    let path = entry.path();
    if path.is_dir() {
      find(&path, files);
    } else if path.ends_with("new/benchmark.json") {
      files.push(path);
    }
  }
}

fn json(path: &Path) -> Option<Value> {
  serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn median(dir: &Path) -> Option<f64> {
  json(&dir.join("estimates.json"))?["median"]["point_estimate"].as_f64()
}

/// The spin counts and gaps the threaded benchmarks saved, by benchmark id.
fn extras(perf: &Path) -> BTreeMap<String, Value> {
  let Ok(read) = fs::read_dir(perf.join("extras")) else {
    return BTreeMap::new();
  };
  read
    .flatten()
    .filter_map(|entry| json(&entry.path()))
    .filter_map(|extra| Some((extra["id"].as_str()?.to_string(), extra)))
    .collect()
}

/// One group: functions down, parameters across. Throughput where an iteration moves more than one
/// record, time per operation otherwise.
fn table(report: &mut String, name: &str, about: &str, entries: &[Entry], extras: &BTreeMap<String, Value>) {
  let per_operation = entries.iter().all(|entry| entry.elements <= 1);
  let mut columns: Vec<Option<String>> = Vec::new();
  let mut rows: Vec<&str> = Vec::new();
  for entry in entries {
    if !columns.contains(&entry.value) {
      columns.push(entry.value.clone());
    }
    if !rows.contains(&entry.function.as_str()) {
      rows.push(&entry.function);
    }
  }
  columns.sort_by(|a, b| by_number(a.as_deref(), b.as_deref()));
  rows.sort_by_key(|function| (rank(function), *function));

  let unit = if per_operation {
    "time per operation, lower is better"
  } else {
    "records per second, higher is better"
  };
  let _ = writeln!(report, "## {name}\n");
  if !about.is_empty() {
    let _ = write!(report, "{about}. ");
  }
  let _ = writeln!(report, "Median, {unit}.\n");
  let header: Vec<&str> = columns.iter().map(|value| value.as_deref().unwrap_or("")).collect();
  let _ = writeln!(report, "| | {} |\n|---|{}", header.join(" | "), "---:|".repeat(columns.len()));

  let cell = |entry: &Entry| {
    if per_operation {
      entry.median
    } else {
      entry.elements as f64 * 1e9 / entry.median
    }
  };
  let lookup = |function: &str, value: &Option<String>| entries.iter().find(|e| e.function == function && &e.value == value);
  let best: Vec<Option<f64>> = columns
    .iter()
    .map(|value| {
      let cells = entries.iter().filter(|e| &e.value == value).map(cell);
      if per_operation {
        cells.min_by(f64::total_cmp)
      } else {
        cells.max_by(f64::total_cmp)
      }
    })
    .collect();
  for function in &rows {
    let cells: Vec<String> = columns
      .iter()
      .zip(&best)
      .map(|(value, best)| {
        let Some(entry) = lookup(function, value) else {
          return "—".to_string();
        };
        let metric = cell(entry);
        let mut text = if per_operation {
          output::duration(metric)
        } else {
          rate(metric)
        };
        if Some(metric) == *best && rows.len() > 1 {
          text = format!("**{text}**");
        }
        if let Some(old) = entry.baseline {
          let _ = write!(text, " ({:+.1}%)", (old / entry.median - 1.0) * 100.0);
        }
        text
      })
      .collect();
    let _ = writeln!(report, "| {function} | {} |", cells.join(" | "));
  }
  report.push('\n');

  let measured: Vec<(String, &Value)> = entries
    .iter()
    .filter_map(|entry| extras.get(&entry.id(name)).map(|extra| (entry.id(name), extra)))
    .collect();
  if !measured.is_empty() {
    spins(report, measured);
  }
}

/// The spin counts and gaps of one group, folded away under its table.
fn spins(report: &mut String, mut measured: Vec<(String, &Value)>) {
  measured.sort_by(|(a, _), (b, _)| a.cmp(b));
  let gaps = measured.iter().any(|(_, extra)| extra["worst_gap_ns"].is_number());
  report.push_str("<details><summary>Spins per record on a full and an empty queue");
  report.push_str(if gaps {
    ", longest wait between two receives</summary>\n\n"
  } else {
    "</summary>\n\n"
  });
  report.push_str(if gaps {
    "| benchmark | full | empty | worst gap |\n|---|---:|---:|---:|\n"
  } else {
    "| benchmark | full | empty |\n|---|---:|---:|\n"
  });
  for (id, extra) in measured {
    let number = |field: &str| extra[field].as_f64().map_or("—".to_string(), |n| format!("{n:.3}"));
    let _ = write!(
      report,
      "| {id} | {} | {} |",
      number("full_per_record"),
      number("empty_per_record")
    );
    if gaps {
      let gap = extra["worst_gap_ns"].as_f64().map_or("—".to_string(), output::duration);
      let _ = write!(report, " {gap} |");
    }
    report.push('\n');
  }
  report.push_str("\n</details>\n\n");
}

fn rate(per_second: f64) -> String {
  match per_second {
    r if r >= 1e9 => format!("{:.2} G/s", r / 1e9),
    r if r >= 1e6 => format!("{:.1} M/s", r / 1e6),
    r => format!("{:.1} K/s", r / 1e3),
  }
}

fn rank(function: &str) -> usize {
  let candidate = function.split('/').next().unwrap_or(function);
  CANDIDATES
    .iter()
    .position(|known| *known == candidate)
    .unwrap_or(CANDIDATES.len())
}

/// Numbers in numeric order, ahead of words, which keep the order they were found in.
fn by_number(a: Option<&str>, b: Option<&str>) -> Ordering {
  match (a.and_then(|a| a.parse::<f64>().ok()), b.and_then(|b| b.parse::<f64>().ok())) {
    (Some(a), Some(b)) => a.total_cmp(&b),
    (Some(_), None) => Ordering::Less,
    (None, Some(_)) => Ordering::Greater,
    (None, None) => Ordering::Equal,
  }
}

/// The machine line every report starts with: numbers without it are not comparable.
fn machine() -> String {
  let read = |path: &str| fs::read_to_string(path).ok().map(|text| text.trim().to_string());
  let run = |program: &str, args: &[&str]| {
    let out = Command::new(program).args(args).output().ok()?;
    out
      .status
      .success()
      .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
  };
  let cpu = read("/proc/cpuinfo")
    .and_then(|info| {
      info
        .lines()
        .find(|line| line.starts_with("model name"))?
        .split(':')
        .nth(1)
        .map(|name| name.trim().to_string())
    })
    .unwrap_or_else(|| env::consts::ARCH.to_string());
  let threads = thread::available_parallelism().map_or(0, usize::from);
  let governor = read("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor").unwrap_or_else(|| "unknown".into());
  let kernel = read("/proc/sys/kernel/osrelease").unwrap_or_else(|| env::consts::OS.into());
  let clock = read("/sys/devices/system/clocksource/clocksource0/current_clocksource").unwrap_or_else(|| "unknown".into());
  let rustc = run("rustc", &["-V"]).unwrap_or_else(|| "rustc unknown".into());
  let commit = run("git", &["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
  let dirty = run("git", &["status", "--porcelain"]).is_some_and(|status| !status.is_empty());

  format!(
    "{cpu}, {threads} logical CPUs, governor {governor}, kernel {kernel}, clocksource {clock}, {rustc}; report made at \
     commit {commit}{}.",
    if dirty { " with uncommitted changes" } else { "" }
  )
}
