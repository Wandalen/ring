# surveyed

Machine-written records of which crates `mutant_survey.sh` last swept clean, and
a digest of the code it swept. `g13_survey_freshness.sh` reads them and fails
when a digest stops matching the crate on disk.

| File | Responsibility |
|------|-----------------|
| `<crate>.surveyed` | The date that crate last swept clean, and the digest of the source and tests that were swept |

One file per crate, written by the survey and **never edited by hand**. There is
no per-crate table here on purpose: unlike `../mutant/` and `../accepted/`,
whose entries each record a distinct judgement worth reading, every record in
this directory says the same mechanical thing about a different crate. A table
listing all 33 would restate the directory listing and go stale on every sweep.

## Why a clean run is the only run that records

The survey writes a record on exit 0 and on no other exit. A crate reporting new
survivors has findings nobody has ruled on; a crate reporting a stale acceptance
has declarations that no longer describe it. Both are unfinished sweeps, and a
record written for either would let G13 report freshness for a crate whose
triage never happened.

That is the same failure the survey's output-clearing exists to prevent, one
level up. A vacuous result at least looks empty; **a stale one arrives carrying
evidence**, and a freshness record is exactly the shape of evidence that would
be believed without being checked.

## What the digest covers, and why both halves

Every `.rs` file under the crate's `src/` and `tests/`, hashed per file and then
reduced to one line — the same walk the survey already takes as its restoration
guard, which is why it costs nothing extra to record.

Both halves are load-bearing, for opposite reasons:

| Half | What changing it means |
|---|---|
| `tests/` | The suite may now kill something a ruling says survives — an acceptance could be stale |
| `src/` | There is code no survey has ever mutated — findings could be missing entirely |

A doc-comment edit to `src/` moves the digest without changing behaviour, and
G13 will ask for a re-survey it did not strictly need. That false positive costs
about a minute for one crate, and the alternative is deciding which source edits
matter by reading them — which is the judgement this gate exists to avoid making.

## Format

A comment header, then `key: value`. G13 reads `digest` and ignores the rest;
`swept` is there for the person reading the failure.

```
crate: ring_stats
swept: 2026-08-30
digest: 1a2ff72fcaeb2804a562bf54e20d011f0d0335b2f9750fa247fdcc92abad783f
```

The digest reproduces independently, which is what stops it being self-referential:

```bash
# must equal the `digest` line in ring_stats.surveyed
( cd ring_stats && find src tests -name '*.rs' -type f | sort | xargs sha256sum ) \
  | sha256sum
```
