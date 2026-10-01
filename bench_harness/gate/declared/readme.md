# declared

Per-family declarations the gates read. Nothing here is code; every file is a
ruling, and every ruling is the reason some gate can fail.

`family.txt` names the family a bare `run_all.sh` grades. Each family has a
directory beside it carrying the same seven core files, plus whatever the extra
gates it declares need of their own — see the tables below.

| File | Responsibility |
|------|-----------------|
| `family.txt` | Which family a bare run grades — one name, overridable with `--family` |
| `exempt.txt` | Crates deliberately graded by no family, each with the reason it is not gated |
| `repo_gates.txt` | The family-independent gates `--every` runs once for the whole repository |
| `ring/` | The 33-crate concurrency write-path family — the only family this repository declares |

`exempt.txt` is read by `g18_family_coverage.sh` and is the only way to reach G18
without gating a crate, which is why its entry count is printed on every run
including a reached one — a gate that goes quiet once green cannot report that it
went green by emptying itself. A line is `crate_name  reason`; a bare name with
no reason is reported as `exemption without a reason` and does not reach, because
an omission wearing an exemption's clothes is the failure this file could
otherwise launder.

**It currently holds zero entries**, and the `ring` family has never had one —
every crate here has always been declared in `ring/crates.txt` rather than
excused from it.

Zero is the reading both gates now print, and neither prints it vacuously. G18
ends `… each claimed exactly once across N famil(ies), 0 exempted`, so the crate
count in that line is the whole tree rather than the tree minus a subtraction
nobody re-checked.

G22 is the one that had to be taught what zero means, because an empty list is
where a negative check stops checking. Every test it makes is a negative — an
entry passes by *not* looking implemented — so a broken detector reports REACHED
precisely when it has stopped working, and an empty file gives it nothing to be
broken against. It answers with two guards, and they cover different failures:

- **The probe.** Before reading a single entry, G22 runs its implementation
  threshold across every crate in the repository, and requires it to
  fire at least once, failing with `the probe is broken, not the tree` if it
  never does. That is a claim about the detector, not about the list, which is
  why it is worth printing on a zero-entry run at all: `probe verified on N
  implemented crate(s)` says the threshold still recognizes an implementation
  somewhere.
- **The byte count.** A comment-only retirement record and a wrong `$GATE_EXEMPT`
  path both parse to zero declared entries, since comments are stripped before
  counting. Raw size tells them apart — the curated file carries its history in
  its header and is never small — so under 200 bytes fails rather than passes.

`exempt.txt` is currently well over six times the floor despite holding no
entries — comfortably the shape the second guard exists to distinguish from an
empty file that happens to read the same, by explaining its own purpose at
enough length to clear it honestly rather than padding to the number.

## Per-family files

Seven core files, carried by every family:

| File | Responsibility |
|------|-----------------|
| `crates.txt` | Family membership, in dependency order — the list every gate iterates |
| `features.txt` | Feature ids the family owns, one per line |
| `gates.txt` | Which gates apply to this family |
| `smoke.txt` | Smoke binaries G7 runs twice and diffs, and diffs again across debug and release |
| `export_surface.txt` | The only crates a consumer outside the family may name as a dependency |
| `unsafe_allowlist.txt` | Crates permitted to opt out of the workspace `unsafe-code = "deny"` |
| `stages.txt` | Which crates and features each stage's own reached-test covers |

Plus one allowlist per repo-wide gate a family declares. It is not declared by
`ring` — `ring/gates.txt` does not name G10 — so the file does not exist
under `ring/` at all:

| File | Responsibility | Read by |
|------|-----------------|---------|
| `pinned_allowlist.txt` | Files permitted to reach libm by either route, each still required to do so | `g10_pinned_math.sh` |

And one optional list `ring/` does declare:

| File | Responsibility | Read by |
|------|-----------------|---------|
| `internal_consumers.txt` | Crates outside the family that may name any of its crates — in-tree tools that measure the internals, each with a reason | `g5_export_surface.sh` |

And four directories, all under `ring/` — the only family this repository
declares:

| Directory | Responsibility | Read by |
|------|-----------------|---------|
| `mutant/` | Historical defects with the exact edit that reinstates each, and the round that found it | `g12_mutation.sh` |
| `accepted/` | Survey survivors ruled decisions rather than gaps, so a sweep reports what is new | `mutant_survey.sh` |
| `surveyed/` | Which crates were last swept clean, and the digest of the code that was swept | `g13_survey_freshness.sh` |
| `corpus_control/` | One fixture crate per corpus gate, each carrying the single defect that gate must name | `run_all.sh --control` |

And one more file, in `ring/` for the same reason:

| File | Responsibility | Read by |
|------|-----------------|---------|
| `corpus_standard.txt` | The doc-corpus standard — the thirteen definitions, the three count floors, the closed tier vocabulary | `g14`-`g17` |

`corpus_standard.txt` is the one declaration here that a gate could plausibly
have carried itself, and the reason it does not is a falsifiable one: editing
`min_findings` from 52 to 53 must flip every currently-closed crate to NOT
REACHED. It does — 13/33 at standard becomes 5/33 — so the thresholds the gates
enforce are the ones written here, not constants that happen to agree with them.

`corpus_control/` inverts every polarity in this directory. Under
`run_all.sh --control` a gate REACHES by *finding* the defect seeded for it, and
a gate reporting the fixture clean has failed. Each gate sees only its own
fixture: pointed at all four, G16 reports a defect whichever of the four is
broken, because a deleted definition also breaks links and a hybrid tier is also
a table disagreement — which reads as four working detectors and proves one.

The first two are the two answers a survivor can get: `mutant/` holds the ones
worth defending, `accepted/` the ones deliberately not — and a survivor in
neither is still decided, just silently. `accepted/` is read by the one thing in
`gate/` that is not a gate, which is why it declares nothing in `gates.txt` and
no family's verdict depends on it directly.

The third is not an answer but a date stamp on the other two, and it exists
because both of them are claims about a crate at the moment it was swept.
`mutant_survey.sh` writes one per crate on a clean run only — a crate with
untriaged survivors, or with an acceptance that no longer matches, has an
unfinished sweep and gets no record. G13 then compares each digest against the
crate on disk, so a suite or a source file that moved under a settled ruling
fails a gate instead of waiting for somebody to think of re-sweeping.

The "each still required to" half of both allowlists is the non-vacuity
condition, not a courtesy: an allowlisted file that stops matching any of its
gate's detectors means the exemption is stale and those detectors have been
proven against nothing, so both gates fail on it. Both gates carry more than one
detector, and one allowlist covers all of them — a file excused from G10's
detector A (`x.sin()`) is excused from its detector B (`mat3x3::from_angle_x`)
too, because the excuse is about what the file is for, not which spelling it
used to get there. That is why these allowlists shrink rather than
accumulate. `mutant/` carries the same condition in mirror image — a declared
`from` block that no longer matches its target fails G12 rather than being
skipped, because a mutation that edits nothing leaves the suite green for the
most misleading reason available.

## Why the gate set is declared per family

G7 (determinism) and G8 (oracle form) were added later and did not exist when
the ring family's gate set first closed at 6/6. Applying them retroactively
would report NOT REACHED against a bar that family's plan never set, which is a
yardstick applied backwards. So `gates.txt` names the applicable set, and
`run_all.sh` echoes it on every run — a family cannot quietly shed a gate,
because dropping one changes the line printed above the first verdict.

## Checking a declaration against the tree

```bash
# every declared crate exists
f=ring
echo "── $f"
grep -vhE '^\s*(#|$)' "$f/crates.txt" \
  | while read -r c; do [ -f "../../../$c/Cargo.toml" ] || echo "  absent: $c"; done
```

**Expected:** nothing after the header. The gates make the same check
themselves — `common.sh`'s `assert_declared_crates_exist` runs before any
gate's own assertion — so this is a way to see the answer without waiting for a
gate run, not a substitute for one.
