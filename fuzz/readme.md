# Fuzzing the lock-free core

`loom` checks thread interleavings; it says nothing about the input space —
capacity edges, claim/publish sequences that wrap the ring, partial batch
grants. These targets feed byte streams of ring operations into `ring_spsc`
and `ring_mpsc` and check every step against a FIFO oracle, so a wrong `Full`,
a lost record, or a reordered drain fails loudly.

## Targets

| Target      | What it drives |
|-------------|----------------|
| `spsc_ops`  | `try_push`, `claim` + write + drop through `Reservation`, `drain`, `drain_up_to` |
| `mpsc_ops`  | `push`, `push_batch`, `claim_batch` + hand write + drop, `claim` + write + drop, `drain`, `drain_up_to` |

Capacities 1/2/4/8/32/128 are picked from the first input byte; every run
ends with a full drain that must empty the oracle. All single-threaded: the
contended interleavings belong to loom.

## Run it

```sh
cargo fuzz run spsc_ops -- -max_total_time=120
cargo fuzz run mpsc_ops -- -max_total_time=120
```

(runs from `fuzz/`; needs the nightly toolchain — see
`.github/workflows/fuzz.yml` for the exact CI invocation).

> Windows note: the prebuilt toolchains ship no AddressSanitizer runtime
> for MSVC, so `cargo fuzz run` (asan by default) cannot start there and
> `-s none` does not link either (missing `sancov` runtime). Build with
> `cargo fuzz build` to check the targets compile, run them for real on
> Linux — which is what CI does.

## Seeds

`fuzz/seeds/<target>/` is checked in and copied into libfuzzer's
`fuzz/corpus/<target>/` before a run. When the fuzzer finds a crash,
minimise it and add the minimised input here with a name saying what it
caught — a regression seed is the proof this harness can fail.
