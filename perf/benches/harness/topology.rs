//! Which logical CPUs share a physical core, read from Linux sysfs, and pinning a thread to one.
//! Elsewhere there are no pairs, and nothing asks to pin.

use std::collections::BTreeMap;
use std::fs;

/// Two logical CPUs on one physical core.
pub fn smt_siblings() -> Option<(usize, usize)> {
  let cores = cores()?;
  let (_, cpus) = cores.iter().rev().find(|(_, cpus)| cpus.len() > 1)?;

  Some((cpus[0], cpus[1]))
}

/// Two logical CPUs on different physical cores of one package.
pub fn separate_cores() -> Option<(usize, usize)> {
  let cores = cores()?;
  let mut last = cores.iter().rev();
  let ((package, _), a) = last.next()?;
  let (_, b) = last.find(|((p, _), _)| p == package)?;

  Some((a[0], b[0]))
}

/// Logical CPUs per `(package, core)`, in ascending order of both. The highest core comes last,
/// and the pairs above take it, because CPU 0 takes most interrupts.
fn cores() -> Option<BTreeMap<(u32, u32), Vec<usize>>> {
  let mut cores: BTreeMap<_, Vec<usize>> = BTreeMap::new();
  for entry in fs::read_dir("/sys/devices/system/cpu").ok()? {
    let entry = entry.ok()?;
    let Some(cpu) = entry
      .file_name()
      .to_str()
      .and_then(|n| n.strip_prefix("cpu"))
      .and_then(|n| n.parse().ok())
    else {
      continue;
    };
    let read = |file: &str| -> Option<u32> {
      fs::read_to_string(entry.path().join("topology").join(file))
        .ok()?
        .trim()
        .parse()
        .ok()
    };
    // An offline CPU has no topology directory.
    if let (Some(package), Some(core)) = (read("physical_package_id"), read("core_id")) {
      cores.entry((package, core)).or_default().push(cpu);
    }
  }
  cores.values_mut().for_each(|cpus| cpus.sort_unstable());

  (!cores.is_empty()).then_some(cores)
}

/// Pin the calling thread to logical CPU `cpu`; `None` leaves it to the scheduler.
///
/// # Panics
///
/// When the CPU refuses the thread: a pinned benchmark that runs unpinned measures something else.
pub fn pin(cpu: Option<usize>) {
  if let Some(id) = cpu {
    assert!(
      core_affinity::set_for_current(core_affinity::CoreId { id }),
      "cannot pin to CPU {id}"
    );
  }
}
