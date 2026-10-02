//! The ring family's write-path comparison, run and printed.
//!
//! ```sh
//! cargo run -p ring_bench --all-features --example comparison
//! ```
//!
//! This is the one place the family's measurement is shown to a human. The
//! library deliberately prints nothing. Every quantity is a field with an
//! accessor, so the suite can assert on values rather than on captured stdout.
//! That leaves the report exercised only by substring assertions, and the first
//! time someone read it end-to-end it exposed four documentation errors
//! (`tests/manual/readme.md` B1). This example exists so that reading is a
//! command rather than a throwaway probe.
//!
//! Without `--all-features` the off-the-shelf candidate is absent and the
//! comparison has five rows instead of six.

use ring_bench::{Comparison, Workload};
use ring_factory::RingConfig;

fn main() {
  // Room for everything: 4096 slots, 256 records. Every candidate keeps the
  // whole workload, so every candidate is eligible to be fastest.
  let roomy = Workload::new(RingConfig::new(4096).unwrap())
    .with_records_per_producer(256)
    .unwrap()
    .with_batch(32)
    .unwrap();

  // 16 slots for 256 records. Every candidate loses, and they lose in three
  // different ways. That is the point of the row.
  let cramped = Workload::new(RingConfig::new(16).unwrap())
    .with_records_per_producer(256)
    .unwrap()
    .with_batch(32)
    .unwrap();

  // Four producers. Most candidates cannot be reached at all through the
  // export Contract, and the refusal list is the larger half of the output.
  let parallel = Workload::new(RingConfig::new(4096).unwrap())
    .with_producers(4)
    .unwrap()
    .with_records_per_producer(256)
    .unwrap()
    .with_batch(32)
    .unwrap();

  section("A — room for everything", "every candidate keeps all 256 records");
  println!("{}", Comparison::run(roomy).report());

  section("B — 256 records into 16 slots", "three different ways to lose");
  println!("{}", Comparison::run(cramped).report());
  println!(
    "  reading: `silent` is the gap between what the write API reported and what\n\
     \x20          the drain produced. Two candidates report 256 successes and keep 16.\n\
     \x20          `tls_over_ring` keeps nothing at all — a rejected first flush leaves\n\
     \x20          its records staged, and the harness does not retry, so every later\n\
     \x20          append fails. Staging turns a shortfall into a stall."
  );

  section("C — four producers", "the export Contract caps what the structures do not");
  println!("{}", Comparison::run(parallel).report());
  println!(
    "  reading: four of six candidates refuse. `contract_ring` and `off_the_shelf`\n\
     \x20          sit on multi-producer structures; the cap is the door, because\n\
     \x20          `ring_handle::Producer` exposes no `try_clone`."
  );

  section("D — the same workload ten times", "why no test asserts an ordering");
  stability();
}

fn section(title: &str, subtitle: &str) {
  println!("\n{}", "─".repeat(78));
  println!("{title}  —  {subtitle}");
  println!("{}\n", "─".repeat(78));
}

/// Ten runs of one workload, reporting how much the winner moves.
///
/// The library refuses to assert an ordering. This shows the measurement that
/// justifies the refusal rather than asking the reader to take it on trust.
fn stability() {
  let mut winners: Vec<&'static str> = Vec::new();
  let mut lowest = u128::MAX;
  let mut highest = 0;

  for _ in 0..10 {
    let workload = Workload::new(RingConfig::new(4096).unwrap())
      .with_records_per_producer(256)
      .unwrap()
      .with_batch(32)
      .unwrap();
    let comparison = Comparison::run(workload);

    let fastest = comparison.fastest().expect("4096 slots hold 256 records");
    if !winners.contains(&fastest.candidate().name()) {
      winners.push(fastest.candidate().name());
    }

    // Track one candidate's own spread across identical runs. Whichever
    // candidate is slowest is uninteresting here; the point is the range.
    for outcome in comparison.outcomes() {
      if outcome.candidate().name() == "contract_ring" {
        lowest = u128::min(lowest, outcome.write_nanos());
        highest = u128::max(highest, outcome.write_nanos());
      }
    }
  }

  println!(
    "  distinct winners across 10 identical runs : {} {:?}",
    winners.len(),
    winners
  );
  println!("  contract_ring's own spread                : {lowest} – {highest} ns");

  let ratio = if lowest > 0 {
    highest as f64 / lowest as f64
  } else {
    f64::NAN
  };
  println!("  ratio                                     : {ratio:.1}x");
  println!(
    "\n  reading: one candidate's run-to-run spread is far larger than the gap\n\
     \x20          between the leading candidates in any single run. That is why no\n\
     \x20          test in this crate asserts which candidate is fastest. The coarse\n\
     \x20          verdict below is docs/decisions/002's own recorded finding, not\n\
     \x20          something this run computed: the three single-producer lock-free\n\
     \x20          paths beat the mutex baseline by several times, with no overlap.\n\
     \x20          `direct_mpsc` is excluded from that verdict — see docs/decisions/002."
  );
}
