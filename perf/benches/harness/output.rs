//! Where results go: criterion's directory, and this suite's own beside it for what criterion has
//! no column for — spin counts, gaps, latency percentiles.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::{env, fs};

/// Cargo's target directory, found the way criterion finds it: `CARGO_TARGET_DIR`, else
/// `cargo metadata`, else `target`. Asked once per process.
pub fn target_dir() -> PathBuf {
  static DIR: OnceLock<PathBuf> = OnceLock::new();
  DIR
    .get_or_init(|| {
      if let Some(dir) = env::var_os("CARGO_TARGET_DIR") {
        return dir.into();
      }
      let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
      Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .ok()
        .and_then(|out| serde_json::from_slice::<serde_json::Value>(&out.stdout).ok())
        .and_then(|metadata| metadata["target_directory"].as_str().map(PathBuf::from))
        .unwrap_or_else(|| "target".into())
    })
    .clone()
}

/// Criterion's results: `CRITERION_HOME`, else `criterion/` under the target directory.
pub fn criterion_dir() -> PathBuf {
  env::var_os("CRITERION_HOME").map_or_else(|| target_dir().join("criterion"), PathBuf::from)
}

/// This suite's results, `perf/` under the target directory.
pub fn perf_dir() -> PathBuf {
  target_dir().join("perf")
}

/// The spin-count file of the benchmark `id`.
pub fn extras_file(dir: &Path, id: &str) -> PathBuf {
  let name: String = id
    .chars()
    .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
    .collect();

  dir.join("extras").join(format!("{name}.json"))
}

/// The latency results file.
pub fn latency_file(dir: &Path) -> PathBuf {
  dir.join("latency.json")
}

/// Write `contents` to `path`, creating its directory.
///
/// # Panics
///
/// When the file cannot be written: a result that silently goes nowhere is worse than a stop.
pub fn save(path: &Path, contents: &str) {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).unwrap_or_else(|e| panic!("cannot create {}: {e}", parent.display()));
  }
  fs::write(path, contents).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
}

/// Whether criterion was asked to run each benchmark once, untimed: `--test`. Nothing is saved then.
pub fn quick() -> bool {
  env::args().any(|arg| arg == "--test")
}

/// `ns` nanoseconds in the unit that keeps it readable: `85.3 ns`, `4.21 µs`, `1.07 ms`.
pub fn duration(ns: f64) -> String {
  match ns {
    n if n < 1e3 => format!("{n:.1} ns"),
    n if n < 1e6 => format!("{:.2} µs", n / 1e3),
    n if n < 1e9 => format!("{:.2} ms", n / 1e6),
    n => format!("{:.2} s", n / 1e9),
  }
}
