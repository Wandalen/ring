# perf

Benchmark suite — the ring family against off-the-shelf queues, under one driver.

Not a family crate: bench, example and test targets only, no `src/`. Exempted from G18
(`bench_harness/gate/declared/exempt.txt`) and allowed past G5 by
`bench_harness/gate/declared/ring/internal_consumers.txt`.

## Running

```sh
./verb/bench suite::all                  # every suite, then the comparison tables
./verb/bench suite::mpsc filter::mutex   # one suite, filtered by benchmark id
./verb/bench suite::report out::cmp.md   # tables from the last results, to a file
./verb/bench suite::all quick::1         # every case once, validated, no numbers
./verb/bench suite::spsc save::before    # on the other commit: baseline::before, then
./verb/bench suite::report baseline::before
```

`cargo bench` builds with the root `[profile.bench]`: fat LTO, one codegen unit. Results land in
`<target>/criterion/` (criterion's, with HTML) and `<target>/perf/` (spin counts, gaps, latency).

## Candidates

| Name           | Queue                        | API measured                                              | Producers |
|----------------|------------------------------|-----------------------------------------------------------|-----------|
| `spsc`         | `ring_spsc`                  | `try_push`; `drain_up_to`, read through `TypedSlot::get`  | 1         |
| `mpsc`         | `ring_mpsc`                  | `push` from copies of one producer; `drain_up_to`         | any       |
| `rtrb`         | `rtrb` 0.4                   | `push`, `push_partial_slice`; `pop`, `read_chunk`         | 1         |
| `sync_channel` | `std::sync::mpsc::sync_channel` | `try_send`; `try_recv` — the standard library's bounded channel, crossbeam-channel's algorithm since 1.67, what a user reaches for first | any |
| `arrayqueue`   | `crossbeam_queue::ArrayQueue` | `push`; `pop` — a bounded MPMC array queue with a stamp per slot: the layout P6 proposes, and `ring_core`'s interim `crossbeam` backend | any |
| `mutex`        | `Mutex<VecDeque<T>>`         | one lock per push, per pop, or per batch                  | any       |

Every queue gets the same power-of-two capacity and record type.

## What is measured

Benchmark ids read `<group>/<candidate>/<mode>/<parameter>`. A mode names how each side moves
records: `push1` one per push, `push32` 32 per push (only where the crate has a batch push);
`pop1` one per pop, `popN` everything available.

| Target    | Group                 | Measures                                                                                                                      | Parameter                            |
|-----------|-----------------------|-------------------------------------------------------------------------------------------------------------------------------|--------------------------------------|
| `micro`   | `push_pop`            | one thread: push a record, pop it back                                                                                        | —                                    |
| `micro`   | `push_full`           | one thread: a push refused by a full queue — what a spinning producer pays per retry                                          | —                                    |
| `micro`   | `pop_empty`           | one thread: a pop of an empty queue, `pop1` and `popN` — what a spinning consumer pays                                        | —                                    |
| `micro`   | `fill_drain`          | one thread: fill to capacity, then drain; per record                                                                          | capacity 64 / 1024 / 16384           |
| `spsc`    | `spsc`                | a producer thread and a consumer thread, 2²⁰ records, every mode                                                              | capacity 64 / 1024 / 16384           |
| `spsc`    | `spsc_payload`        | the same at 1024 slots with wider records: protocol cost against copy cost                                                    | record size 8 / 64 / 256 B           |
| `spsc`    | `spsc_pinned`         | the same at 1024 slots, both threads on SMT siblings of one core, then on two cores                                           | `siblings` / `cores`                 |
| `batch`   | `batch`               | one producer pushing that many records per operation, 1024 slots                                                              | 1 / 8 / 32 / 128                     |
| `mpsc`    | `mpsc_producers`      | 2²⁰ records split over that many producer threads, 1024 slots                                                                 | 1, 2, 4, 8, cores − 1                |
| `mpsc`    | `mpsc_capacity`       | four producers against 64 slots, always full, and 16384, never full                                                           | capacity                             |
| `mpsc`    | `mpsc_oversubscribed` | twice as many producers as logical CPUs, so producers are preempted mid-operation; adds the longest wait between two receives | producers                            |
| `latency` | `pingpong`            | one-way hand-off: round trips over two queues, timed in bulk and halved                                                       | —                                    |
| `latency` | `steady`              | percentiles of receive time minus scheduled send time under an offered load                                                   | 1 / 4 / 8 producers × 1 / 5 / 10 M/s |
| `latency` | `oversubscribed`      | the same from twice as many producers as logical CPUs                                                                         | 1 / 5 M/s                            |
| `latency` | `bursts`              | 64 records per producer, every producer at once, every 200 µs                                                                 | 1 / 4 / 8 producers                  |

Threaded runs build a fresh queue per run and time from a spinning start line to the consumer's
last record; building, spawning and validation stay outside the clock. Producers spin on a full
queue and the consumer on an empty one; both counts per record follow criterion's lines as
`spins:`. The consumer checksums every record and checks per-producer order, and wide records
end-to-end: a lost, duplicated, reordered or torn record panics with the candidate's name and no
number is reported (`tests/harness_test.rs` plants each).

Latency runs are open-loop: each record is sent at its scheduled moment and stamped with that
moment, so a push held up by a full queue counts against the records it delayed. The first 200 ms
are delivered and checked but not recorded. A candidate delivering under 95 % of the offered rate
is reported as saturated, without percentiles. The output starts with what one clock read costs on
the machine, since every latency carries up to one: tens of nanoseconds with a TSC clocksource,
over a microsecond with HPET. `mpsc_oversubscribed` reads the clock once per pop as well, for its
gaps.

## Comparison tables

`./verb/bench suite::report` (`cargo run -p perf --example report`) prints markdown: a machine
line, then one table per group — candidates and modes down, parameters across, the median as
records per second or time per operation, the best of each column in bold — with the spin counts
and gaps folded under it, then the latency tables. `baseline::<name>` adds each cell's change
against a criterion baseline saved with `save::<name>`.

## Adding a candidate

Implement `Candidate` in `benches/harness/candidates.rs`: `new`, and `split` handing `Tx` / `Rx`
ends to a `Run`. Use the crate's fastest idiomatic API and name it in the doc comment; set
`PUSH_BATCH` only for a real batch push. Add it to the bench functions and to
`tests/harness_test.rs`.
