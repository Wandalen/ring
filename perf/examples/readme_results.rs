//! The root readme's measured-results table, from the last criterion runs: the same medians
//! `report` reads, reduced to the rows the readme's `Measured results` section claims, spliced
//! back into the readme between its `measured-results` markers.
//!
//! ```sh
//! cargo run -p perf --example readme_results -- [--readme <path>] [--out <file>] [--stdout]
//! ```
//!
//! Without `--out` or `--stdout` the table replaces what stands between the markers in the readme,
//! `readme.md` at the repository root unless `--readme` says otherwise. A row whose benchmark ids
//! the last run did not produce is left out with a warning on stderr, never estimated; a run that
//! produced none of the rows changes nothing.

#[path = "../benches/harness/mod.rs"]
mod harness;

use std::path::{Path, PathBuf};
use std::{env, fs};

use harness::output;
use harness::readme::{self, Id, Row};
use serde_json::Value;

/// The readme table's rows, in its order. The last two read the `mpsc-primary` candidate the
/// primary-handle spike benches (open #21); until that lands in this tree they are skipped with a
/// warning.
const ROWS: [Row; 6] = [
  Row {
    claim: "`ring_spsc`: two-thread throughput, 16384 slots",
    inhouse: &[Id {
      group: "spsc",
      function: "spsc/push1_popN",
      value: Some("16384"),
    }],
    alternative: &[Id {
      group: "spsc",
      function: "rtrb/push1_popN",
      value: Some("16384"),
    }],
  },
  Row {
    claim: "`ring_spsc`: two-thread throughput, 1024 slots",
    inhouse: &[Id {
      group: "spsc",
      function: "spsc/push1_popN",
      value: Some("1024"),
    }],
    alternative: &[Id {
      group: "spsc",
      function: "rtrb/push1_popN",
      value: Some("1024"),
    }],
  },
  Row {
    claim: "`ring_spsc`: 8-byte records, two-thread",
    inhouse: &[Id {
      group: "spsc_payload",
      function: "spsc/push1_popN",
      value: Some("8"),
    }],
    alternative: &[Id {
      group: "spsc_payload",
      function: "rtrb/push1_popN",
      value: Some("8"),
    }],
  },
  // The sweep's producer counts are the machine's, so the row reads every value the group has and
  // renders the range across them.
  Row {
    claim: "`ring_mpsc`: batched producers (`push32`), 1–13 producers",
    inhouse: &[Id {
      group: "mpsc_producers",
      function: "mpsc/push32_popN",
      value: None,
    }],
    alternative: &[Id {
      group: "mpsc_producers",
      function: "mutex/push32_popN",
      value: None,
    }],
  },
  // The ordinary handle's one-producer row measures the same shape the two-thread `spsc` group
  // does: one producer, one consumer, 1024 slots, `push1_popN`.
  Row {
    claim: "`ring_mpsc`: primary handle, `push1_popN`, 1024 slots",
    inhouse: &[Id {
      group: "spsc",
      function: "mpsc-primary/push1_popN",
      value: Some("1024"),
    }],
    alternative: &[Id {
      group: "mpsc_producers",
      function: "mpsc/push1_popN",
      value: Some("1"),
    }],
  },
  Row {
    claim: "`ring_mpsc`: primary handle, single-thread fill/drain",
    inhouse: &[Id {
      group: "fill_drain",
      function: "mpsc-primary/push1_popN",
      value: Some("1024"),
    }],
    alternative: &[Id {
      group: "fill_drain",
      function: "mpsc/push1_popN",
      value: Some("1024"),
    }],
  },
];

struct Args {
  /// The readme to splice, `readme.md` at the repository root unless `--readme` says otherwise.
  readme: PathBuf,
  /// Write the fragment here instead of splicing.
  out: Option<PathBuf>,
  /// Print the fragment instead of splicing.
  stdout: bool,
}

impl Args {
  fn parse() -> Self {
    let mut args = Self {
      readme: PathBuf::from("readme.md"),
      out: None,
      stdout: false,
    };
    let mut words = env::args().skip(1);
    while let Some(word) = words.next() {
      match word.as_str() {
        "--readme" => args.readme = value(&mut words, "--readme").into(),
        "--out" => args.out = Some(value(&mut words, "--out").into()),
        "--stdout" => args.stdout = true,
        "--help" | "-h" => {
          println!("readme_results [--readme <path>] [--out <file>] [--stdout]");
          std::process::exit(0);
        }
        flag if flag.starts_with('-') => {
          eprintln!("readme_results: unknown flag {flag}");
          std::process::exit(2);
        }
        word => {
          eprintln!("readme_results: unexpected argument {word}");
          std::process::exit(2);
        }
      }
    }

    args
  }
}

/// The word after `flag`, or exit: a valueless flag is a usage error, not a default.
fn value(words: &mut impl Iterator<Item = String>, flag: &str) -> String {
  words.next().unwrap_or_else(|| {
    eprintln!("readme_results: {flag} needs a value");
    std::process::exit(2);
  })
}

fn main() {
  let args = Args::parse();
  let criterion = output::criterion_dir();
  let data = results(&criterion);
  let (fragment, skipped) = readme::render(&ROWS, &data, &output::machine());
  for row in &skipped {
    eprintln!("readme_results: {row}");
  }
  if skipped.len() == ROWS.len() {
    eprintln!(
      "readme_results: no criterion data for any readme row under {}",
      criterion.display()
    );
    std::process::exit(1);
  }

  if let Some(path) = &args.out {
    output::save(path, &fragment);
  } else if args.stdout {
    print!("{fragment}");
  } else {
    let text = fs::read_to_string(&args.readme).unwrap_or_else(|e| stop(format!("cannot read {}: {e}", args.readme.display())));
    let spliced = readme::splice(&text, &fragment).unwrap_or_else(|e| stop(format!("{}: {e}", args.readme.display())));
    fs::write(&args.readme, spliced).unwrap_or_else(|e| stop(format!("cannot write {}: {e}", args.readme.display())));
  }
}

/// Exit 2 with `message`: a readme that cannot be read or spliced is a usage problem, not a crash.
fn stop(message: String) -> ! {
  eprintln!("readme_results: {message}");
  std::process::exit(2);
}

/// Every benchmark's median under criterion's directory, keyed the way `report` prints ids:
/// `<group>/<function>/<value>`. What a filtered run skipped simply is not there.
fn results(criterion: &Path) -> readme::Data {
  let mut files = Vec::new();
  find(criterion, &mut files);
  let mut data = readme::Data::new();
  for file in files {
    let Some(new_dir) = file.parent() else {
      continue;
    };
    let (Some(benchmark), Some(median)) = (json(&file), median(new_dir)) else {
      continue;
    };
    let text = |field: &str| benchmark[field].as_str().map(str::to_string);
    let (Some(group), Some(function)) = (text("group_id"), text("function_id")) else {
      continue;
    };
    let key = match text("value_str") {
      Some(value) => format!("{group}/{function}/{value}"),
      None => format!("{group}/{function}"),
    };
    data.insert(
      key,
      readme::Measurement {
        elements: benchmark["throughput"]["Elements"].as_u64().unwrap_or(1),
        median_ns: median,
      },
    );
  }

  data
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
