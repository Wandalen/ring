# Shared helpers for the stage gates. Sourced, never executed.
#
# Every gate exits 0 (REACHED) or non-zero (NOT REACHED). No gate may pass
# vacuously. Each pairs its assertion with a non-vacuity check so that an empty
# workspace reads "not reached" rather than "nothing to check".
#
# The gates were once family-specific in their code and are now family-neutral
# in both code and declaration. What changed and why:
#
#   * `ring_crates()` used to glob a shared parent directory for anything
#     prefixed `ring_`. A glob reports whatever is on disk, so it cannot
#     distinguish "the family is complete" from "the family does not exist". G5
#     and G6, both conjunctions over a declared set, read REACHED against a
#     family with zero crates in it. Membership is now an explicit list, and
#     `assert_declared_crates_exist` checks it against the tree, so a
#     declaration naming a crate that is not there fails loudly.
#
#   * FEATURE_LO / FEATURE_HI were two constants in this file, so a second
#     family could not be declared without editing the code the first family
#     runs on. Feature ids are now a list in the family's own declaration.
#
#   * Which gates apply is itself declared, in each family's own `gates.txt`.
#     See `declared/ring/gates.txt` for why G7 and G8 are absent here. They were
#     added after this family's own gate set first closed, and applying them
#     retroactively would report NOT REACHED for a bar its plan never set.

REPO="$( cd "$( dirname "${BASH_SOURCE[0]}" )/../../.." && pwd )"
GATE_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
DECL_ROOT="$GATE_DIR/declared"

# Where crates live, and how these gates find themselves.
#
# The declarations are located from BASH_SOURCE rather than from a literal under
# a shared parent directory, because they are a sibling of this file and move
# with it. A literal did not survive the ring family moving to its own root.
# Every gate then aborted looking for a declaration directory one tree over.
#
# Crates can sit under more than one root: this repo's own crates under one
# root, and (in the layout this tooling was originally built for) sibling roots
# holding other crate families. So a single root cannot address them all. A
# demo-binaries root is deliberately not a crate root here. It was outside the
# census before a later crate relocation, and adding it would widen what G18 and
# G22 grade without notice. `strip_crate_root` below also relies on paths under
# it staying absolute so that `family_bin_sources` can drop them from G1's join.
# Where a declared name may be *found* is a separate question with a separate
# answer; see RESOLVE_ROOTS.
CRATE_ROOTS=( "$REPO/module" "$REPO/ring" "$REPO/substrate" )

# Where a declared crate name may be found, as opposed to what the census
# counts. Conflating the two is what made `assert_declared_crates_exist` report
# a batch of crates as "absent from the tree" while every one of them sat under
# the demo-binaries root instead. A relocation moved a set of `demo_*` crates
# out of their old root, the families still declaring them as members kept their
# old reference, and `crate_dir` searched the census roots only. Every affected
# gate invocation aborted on that false verdict without grading anything.
#
# Only `crate_dir` reads this. `all_crate_names` and `strip_crate_root` go on
# reading CRATE_ROOTS, so a name becoming resolvable does not widen the census
# and does not make a path under that root relative. A declared crate there now
# resolves and is still counted by nobody.
RESOLVE_ROOTS=( "${CRATE_ROOTS[@]}" "$REPO/spike" )

# Roots that place a grouping segment between themselves and a crate:
# `<root>/<family>/<crate>`, where `<family>` organizes the root and is not part
# of the crate's own address. Declared explicitly, not inferred, because once a
# root prefix is stripped a nesting root and a flat one are indistinguishable by
# shape. A family directory's own crate-shaped name reads exactly like an
# ordinary crate's source path. Guessing is what broke every gate, without a
# word, the one time a whole root was promoted to sit one level differently than
# before. The affected tree fell outside CRATE_ROOTS, `crate_dir` resolved none
# of its crates, and every per-family gate aborted at
# `assert_declared_crates_exist` without grading anything.
NESTED_ROOTS=( "$REPO/substrate" )

# Every cargo workspace root in the tree, absolute, repo root first.
#
# The tree can be more than one workspace. That is the same fact crate_dir()'s
# tie-break below is built on, stated here as something callable instead of as a
# remark. `$REPO/Cargo.toml` is one workspace, covering the repo's own top-level
# crates; a sibling root's own `Cargo.toml` can be an independent virtual
# `[workspace]` of its own.
#
# This matters to anything that asks cargo a question, because `cargo metadata`
# answers only for the workspace it is run in. A single invocation at `$REPO`
# reports only the root workspace's own packages and omits every crate held by a
# sibling workspace. It does so without an error, as a shorter list. Asking once
# and reading the answer as "the crates" is therefore asking about a fraction of
# the tree. Ask once per root and merge.
#
# A `[workspace]` guard, mirroring all_crate_names()'s `[package]` one. Nothing
# requires a directory under `substrate/` to be a workspace, and a family that
# became an ordinary package would otherwise be handed to `cargo metadata` as a
# root, where it answers for whichever workspace encloses it instead.
workspace_roots() {
  printf '%s\n' "$REPO"
  find "$REPO/substrate" -mindepth 2 -maxdepth 2 -name Cargo.toml \
    -exec grep -lE '^\[workspace\]' '{}' '+' 2>/dev/null \
    | sed 's|/Cargo\.toml$||' | sort
}

# Absolute directory of a crate; non-zero and silent if it is in no root.
#
# A crate sits at one of three depths under a root: flat (`ring/ring_align`, or
# an ungrouped crate directly under a root), one family directory deeper (a
# sibling family's own crate nested one level under its family name), or one
# further grouping directory deeper still, the shape a later crate-nesting
# relocation introduced. Checked flat first, then nested, root by root, first
# match winning.
#
# That tie-break does real work, because the roots are not one workspace. Some
# roots are members of the repo-root workspace, while a sibling root's own
# `Cargo.toml` can be a workspace of its own, so cargo's
# one-package-name-per-workspace rule does not reach across them. Two roots can
# hold same-named crates and both will build, and cargo will never complain.
# This function resolves such a pair by RESOLVE_ROOTS order alone, silently. It
# answers with whichever root comes first, not with the copy the caller meant,
# and there is no signal that a choice was made at all.
#
# That was not hypothetical. One same-named crate was once found on disk in
# three different roots, byte-identical, until the duplicate copies were deleted
# and their dependents repointed at the single surviving one. `all_crate_names`
# now counts no duplicate among them, so the tie-break currently has nothing to
# break a tie on. Keep it anyway. Nothing in this file, and nothing in cargo,
# prevents the next such pair from appearing, and the failure it produces is a
# wrong answer, not an error.
#
# Both searches require a `[package]` manifest in the matched directory instead
# of taking any directory of the right name, and the bar is `[package]`
# specifically, not a Cargo.toml of any kind. The two clauses are there for
# different reasons.
#
# For the nested search, searching from depth 2 puts ordinary source directories
# in range, such as `some_root/example_crate/src` and `ring/ring_align/tests`.
# None of those is spelled like a crate today, but nothing enforces that. A
# false hit resolves a crate to a directory holding no manifest at all, which
# every caller downstream reads as the crate existing.
#
# For the flat search the failure was live, not hypothetical. `[ -d ]` alone
# answered with `<root>/<family>` for every family whose directory is spelled
# like one of its own member crates, and several are. So the workspace directory
# shadowed the crate one level below it and won, being flat. Those resolved to a
# directory with no `src/`, no `tests/` and no `docs/`, and a `Cargo.toml`
# holding `[workspace]` and nothing else. Callers do not notice. Most append a
# subpath and test it, so `g6_unsafe.sh` skipped the crate from the unsafe audit
# entirely, `g5_export_surface.sh` counted zero public items, `g3_features.sh`
# dropped the crate's tests from the feature search and `g4_manual.sh` reported
# a manual plan that exists as missing. Requiring `[package]` makes the flat
# clause fail for a workspace directory, so resolution falls through to the
# nested search directly below, which finds the real crate at depth 2.
crate_dir() {
  local root hit
  for root in "${RESOLVE_ROOTS[@]}"; do
    if grep -qE '^\[package\]' "$root/$1/Cargo.toml" 2>/dev/null; then
      printf '%s\n' "$root/$1"; return 0
    fi
    hit="$( find "$root" -mindepth 2 -maxdepth 3 -type d -name "$1" \
      -exec grep -qE '^\[package\]' '{}/Cargo.toml' ';' -print 2>/dev/null | head -1 )"
    [ -n "$hit" ] && { printf '%s\n' "$hit"; return 0; }
  done
  return 1
}

# Absolute path of the cargo workspace that owns a crate; non-zero and silent
# if the crate resolves to no root, or sits under no `[workspace]` at all.
#
# Needed because this tree can be more than one workspace. `$REPO/Cargo.toml`
# covers the repo's own top-level crates; a sibling root's own `Cargo.toml` can
# be an independent virtual workspace, and `cargo metadata` at `$REPO` does not
# see a single one of the crates it holds. Any caller that hardcodes
# `cd "$REPO"` before a `cargo -p <crate>` call therefore works for crates in
# the root workspace and fails for crates in a sibling one. It fails in the
# shape that reads as a defect in the crate. `cargo build -p <crate>` at `$REPO`
# answers "package ID specification did not match any packages", and the same
# package asked for its bin targets answers with nothing. That is
# indistinguishable from a crate that genuinely declares none.
#
# Walks up from the crate directory instead of pattern-matching the path, so it
# stays correct for whatever nesting the next relocation introduces. Stops at
# `$REPO` because that is the outermost workspace. Continuing past it would
# leave the repository entirely.
#
# Depends on crate_dir() answering with a crate, not a workspace. Given a
# sibling family's own directory (a `[workspace]` with no `[package]`), the walk
# would start one level too high and answer `$REPO`, silently, for a crate whose
# real owner is that family's own root. That is why crate_dir()'s flat clause
# requires `[package]`; see its own header.
crate_workspace_root() {
  local d
  d="$( crate_dir "$1" )" || return 1
  [ -n "$d" ] || return 1
  while [ "$d" != "/" ]; do
    d="$( dirname "$d" )"
    if grep -qE '^\[workspace\]' "$d/Cargo.toml" 2>/dev/null; then
      printf '%s\n' "$d"; return 0
    fi
    [ "$d" = "$REPO" ] && break
  done
  return 1
}

# The family's crates grouped by the workspace that owns them, one line per
# group, crates space-separated after the root:
#
#   /abs/path/to/<root>/<family>  crate_one crate_two …
#
# What this is for: a gate that builds `-p <crate>` arguments must run cargo
# from a workspace that contains those packages. Every `-p` call site here
# hardcoded `cd "$REPO"`, which is legal only for crates in the root workspace.
# Point it at a family living under a sibling root's own `<root>/<family>/` and
# cargo refuses the invocation whole:
#
#   error: cannot specify features for packages outside of workspace
#
# because `--all-features` / `--no-default-features` resolve features, and
# features can only be resolved for members. Nothing compiles, cargo emits that
# one line and exits non-zero, and the gate reports whatever its own `fail`
# message says. That is a claim about crates cargo never read.
#
# The refusal is a property of the whole selection, not of each package, which
# is why this stayed invisible for years of runs. Cargo accepts the flags as
# soon as ONE selected package is a root member, and then checks the non-member
# ones alongside it. Measured against the live tree: `ring` (33/33 members)
# always cleared that bar and ran for real; families with zero root members
# never did. Some of those were masked one layer further still, failing G2's
# public-item check before reaching a cargo call at all, so only a handful ever
# showed the symptom.
#
# Ordering is deterministic: `sort` on the root path, and `family_crates` is
# itself sorted, so a group's crate order is stable across runs.
#
# `LC_ALL=C` on that sort is required, not hygiene. The awk below closes a group
# the moment the root changes, so it is correct only if equal roots are
# contiguous, and under a UTF-8 locale they are not. Collation there ignores
# punctuation, so `$REPO<TAB>ws_verify` and `$REPO/substrate/exact<TAB>…` are
# compared as `…sandboxws_verify` against `…sandboxsubstrateexact…`, putting the
# substrate line first and splitting `$REPO` into two groups around it. Observed
# on infra, whose 27 crates span seven workspaces: `$REPO` came back twice, and
# the second occurrence would have re-run cargo over a single stray crate.
#
# Falls back to `$REPO` for a crate whose workspace cannot be resolved, for
# smoke_root()'s reason. An unresolvable name should still reach cargo and fail
# there with cargo's own diagnostic, not be silently dropped here.
family_workspace_groups() {
  local c root
  while read -r c; do
    root="$( crate_workspace_root "$c" 2>/dev/null )" || root=""
    [ -n "$root" ] || root="$REPO"
    printf '%s\t%s\n' "$root" "$c"
  done < <( family_crates ) \
  | LC_ALL=C sort \
  | awk -F'\t' '
      $1 != prev { if ( prev != "" ) printf "\n"; printf "%s", $1; prev = $1 }
      { printf " %s", $2 }
      END { if ( prev != "" ) printf "\n" }
    '
}

# One cargo invocation per owning workspace, not one at `$REPO` for the lot.
# The single-root form is only legal for a family whose crates are root members;
# for the six that are not, cargo refused the invocation outright and the caller
# reported a defect about crates it had never compiled. See
# family_workspace_groups() above for the full shape of that failure.
#
# Each workspace's block is labelled in the stderr destination, because a
# diagnostic now has more than one possible origin and "which workspace" is the
# first thing a reader needs. Every group runs even after one fails. A family's
# crates split across two workspaces should report both sets of diagnostics in
# one run, not stop at whichever sorted first.
#
# stdout and stderr are separate parameters rather than one merged log, because
# the two callers need opposite things from them. G2 reads the merged text and
# passes the same path twice. G9 parses stdout as cargo's JSON message stream
# and must keep cargo's own prose out of it, so it passes two paths; merging
# there would put `──── <workspace> ────` labels and `error: could not compile`
# lines into a stream every downstream `jq` expects to be JSON-per-line.
#
# Both destinations are truncated once, before the loop, and appended to
# thereafter. So passing one path for both yields exactly the interleaving
# `>>log 2>&1` produced when this lived in G2, and a caller that passes two
# never sees one truncate the other mid-run.
#
# `$var` is one `NAME=value` environment assignment, or the empty string for a
# caller that wants the environment left alone. `${var:+"$var"}` is what makes
# the empty case work: unquoted at the outer level it contributes zero words
# when `$var` is empty, and the inner quotes keep a value with a space in it
# (`RUSTFLAGS=-D warnings`) a single word when it is not. Plain `env "$var"`
# would pass `''` through as the command name and fail before cargo ran.
#
# The subcommand is its own parameter so the `-p` list can be injected directly
# after it instead of appended at the end. Appending is what this function did
# first, and it is wrong for any caller whose argument list contains cargo's
# `--` separator. Everything past `--` is forwarded to the driver, so G9's
# `clippy … -- -D warnings` put the package flags in front of clippy-driver,
# which answered `error: Unrecognized option: 'p'` and compiled nothing. G2's
# three calls have no `--` and never saw it. Splitting the subcommand off keeps
# every call site unchanged. `clippy … -- -D warnings` arrives here as `sub`
# plus a tail whose `--` stays exactly where the caller put it.
cargo_over_workspaces() {
  local out="$1" err="$2" var="$3" sub="$4"; shift 4
  local ws pkgs pkg rel rc=0
  local -a pkg_args
  : >"$out"; : >"$err"
  while read -r ws pkgs; do
    [ -n "$ws" ] || continue
    pkg_args=()
    for pkg in $pkgs; do pkg_args+=( -p "$pkg" ); done
    rel="${ws#"$REPO"}"; rel="${rel#/}"; [ -n "$rel" ] || rel="<repo root>"
    printf '──── %s ────\n' "$rel" >>"$err"
    ( cd "$ws" && env ${var:+"$var"} cargo "$sub" "${pkg_args[@]}" "$@" ) >>"$out" 2>>"$err" || rc=1
  done < <( family_workspace_groups )
  return "$rc"
}

# Every crate name on disk across all roots, sorted. The census G18 and G22
# grade the declarations against. Depth ranges over every shape crate_dir()
# resolves: flat (2 levels under a root), family-nested (3) and nested one level
# deeper still (4). Those are confirmed the only depths present in the tree.
#
# A manifest is only a crate's if it declares a `[package]`. Every sibling
# root's own `<root>/<family>/Cargo.toml` is a virtual `[workspace]` manifest holding
# no package at all, and it sits at exactly the depth a flat crate's own
# manifest does, so counting manifests alone would enter every family
# directory name into the census as a crate that does not exist. G18 would
# then report each as unclaimed by any family, and no crates.txt could ever
# claim one without `crate_dir` failing on it in the same run.
all_crate_names() {
  find "${CRATE_ROOTS[@]}" -mindepth 2 -maxdepth 4 -name Cargo.toml \
    -exec grep -lE '^\[package\]' '{}' '+' 2>/dev/null \
    | sed 's|/Cargo\.toml$||; s|.*/||' | sort
}

# Strip whichever crate root prefixes each path on stdin, leaving `<crate>/…`.
#
# Declarations name a crate and a path inside it, never the root that crate
# currently sits under: `<crate>/tests/some_test.rs`, not `<root>/<crate>/…`.
# That is deliberate. It is the property that lets a family move between roots
# without invalidating every declaration naming it. The ring family's move did
# exactly that to every path literal that was not written this way.
#
# Several gates also key-match two independently derived lists against each
# other: G1 joins its per-file coverage rows against the binary-source list, and
# G10 matches its detector hits against the pinned allowlist. Both sides must
# reduce to the same spelling or the join silently matches nothing, which reads
# as "no binaries to drop" and "no allowlisted file still reaching libm" rather
# than as a broken gate.
#
# A crate may also sit one grouping directory deeper than its root (see
# crate_dir()); that intermediate segment is stripped too, so every shape
# reduces to the same `<crate>/…` a declaration names.
#
# NESTED_ROOTS is consumed first, and deliberately so. Its rules take the root
# prefix and the grouping segment off together. Were the plain-root loop to run
# first, a nested root's own `<root>/<group>/<crate>/…` would reduce to
# `<group>/<crate>/…`, which the trailing grouping rule no longer matches. The
# literal root segment it keys on is exactly what the root rule just removed.
# Every such path would then carry a group segment no declaration names, and
# every join keyed on that spelling would match nothing.
#
# The trailing rule also covers a grouping segment that sits *inside* a plain
# root instead of being a root of its own, and input that arrives already
# repo-relative. strip_crate_root is also fed tool output, not only absolute
# filesystem paths.
#
# This function does not know about a demo-binaries root, so it leaves a path
# under one absolute. That was once described here as deliberate.
# family_bin_sources() dropped whatever this left absolute, and the note claimed
# that was how paths outside every crate root got excluded from G1's join. It
# had stopped being true. A later crate relocation moved a batch of `demo_*`
# crates into that root, turning that "exclusion" into blindness to every binary
# they own. G1 then held those binaries to a coverage bar no test can reach, and
# never budget-checked them. One relocated demo's own main.rs ran well over a
# thousand lines against a 12-line shim budget. family_bin_sources() now builds
# its key from cargo's own manifest_path and calls nothing here, so no part of
# G1 depends on this behavior any more.
#
# Whether the remaining callers want it is a separate, open question, not an
# endorsement. g10_pinned_math.sh normalizes its hits through this function and
# classifies them against an allowlist keyed on the stripped spelling, so adding
# a rule for that root would move that key space under it. G10 has its own,
# already-filed blindness to that root upstream of this (its source list, not
# its key), so the two want deciding together rather than one patched blind.
strip_crate_root() {
  local root expr=''
  for root in "${NESTED_ROOTS[@]}"; do expr="$expr s#^$root/[^/]+/##;"; done
  for root in "${CRATE_ROOTS[@]}"; do expr="$expr s|^$root/||;"; done
  expr="$expr s#^(substrate|division)/[^/]+/##;"
  sed -E "$expr"
}


# Which family this run grades: the environment, else the declared default.
if [ -z "${GATE_FAMILY:-}" ]; then
  GATE_FAMILY="$( grep -vE '^\s*(#|$)' "$DECL_ROOT/family.txt" 2>/dev/null | head -1 )"
fi
DECL="$DECL_ROOT/$GATE_FAMILY"

# Strip comments and blank lines from a declaration file.
#
# Finding(decl_lines_keeps_a_trailing_cr_on_crlf_input): `grep -v` prints a
# non-comment, non-blank line unchanged and does not trim `\r`. So a
# `crates.txt`/`gates.txt`/`smoke.txt`/`features.txt` saved with CRLF line
# endings would return `"some_crate\r"` instead of `"some_crate"` for every
# accepted line. Every caller downstream of `family_crates`, `family_gates`,
# `family_smoke` and `feature_ids` treats the returned line as the bare declared
# token with no further trim. `crate_dir "$c"` fails a directory check that
# should succeed, `g1_coverage.sh`'s `pkg_args+=( -p "$c" )` hands
# `cargo tarpaulin` a package name cargo can never resolve, and
# `g18_family_coverage.sh`'s `claim["$c"]="…"` keys an associative array on the
# dirty string. That key then never matches a lookup keyed on the clean name
# `all_crate_names` produces. Every one of these reads as
# "declared but absent from the tree", not as a formatting mismatch.
# No live trigger: every file under `declared/` in this repository is confirmed
# LF-only (`grep -clP '\r'` across the whole tree returns zero hits), so this is
# disclosed, not fixed.
decl_lines() {
  grep -vE '^\s*(#|$)' "$1" 2>/dev/null
}

# The family's crates, sorted, or only that stage's crates when a stage scope is
# loaded. Every gate iterates this, so scoping here scopes all of them.
family_crates() {
  if [ -n "${GATE_CRATES:-}" ]; then
    tr ',' '\n' <<<"$GATE_CRATES" | grep -v '^$' | sort
  else
    decl_lines "$DECL/crates.txt" | sort
  fi
}

# Every crate of the family, regardless of stage scope. G5 needs this: a stage
# run still has to know which manifests are inside the family and which are
# outside it, and the answer must not narrow just because one stage is under
# test.
family_members() {
  decl_lines "$DECL/crates.txt" | sort
}

# Feature ids in scope: the family's own list, or the stage's own subset.
feature_ids() {
  if [ -n "${GATE_FEATURES:-}" ]; then
    tr ',' '\n' <<<"$GATE_FEATURES" | grep -v '^$' | sort -n
  else
    decl_lines "$DECL/features.txt" | sort -n
  fi
}

# The gates declared for this family, in order.
family_gates() {
  decl_lines "$DECL/gates.txt"
}

# Smoke binaries G7 diffs.
family_smoke() {
  decl_lines "$DECL/smoke.txt"
}

# Every .rs file under one crate's src/ and tests/, hashed per file and sorted.
# This is the listing `sha256sum -c` reads back. Takes a crate directory, and
# cds into it so the paths stay relative to the crate.
#
# Shared because two callers must agree on it exactly. `mutant_survey.sh` takes
# it before a survey as a restoration guard and records its reduction as the
# crate's freshness digest; `g13_survey_freshness.sh` recomputes that reduction
# and fails when it differs. Two separate walks that drifted apart would compare
# hashes of different file sets and report the difference as staleness. That is
# a gate failing for a reason that has nothing to do with the crate.
#
# Finding(crate_rs_listing_unpaired_find_xargs_splits_whitespace_filenames):
# `find` is not paired with `-print0`/`xargs -0` here. A `.rs` filename
# containing a space, tab, or newline would be split by `xargs`'s default
# whitespace-delimited parsing into two or more bogus paths, e.g.
# `src/weird name.rs` becoming the separate arguments `src/weird` and `name.rs`.
# `sha256sum` reports those as "No such file or directory" on stderr while still
# omitting that file's hash line from stdout, instead of crashing outright. Both
# callers read only stdout through a `$( ... )` capture
# (`g13_survey_freshness.sh`'s `crate_digest`, `mutant_survey.sh`'s
# `hash_targets`) without checking this function's own exit code. So the
# corruption would show up as a silently wrong freshness digest or a silently
# wrong restoration-guard listing, never as a script failure. Unlike a
# crate/package name, Cargo does not restrict `.rs` source file names to a safe
# charset, and `#[path = "weird name.rs"]` is legal. So this is not structurally
# ruled out the way a crate-name-keyed lookup would be. No live trigger:
# confirmed empirically that every `.rs` file under every `src/`/`tests/`
# directory in this repository's own crate roots is free of whitespace and glob
# metacharacters.
crate_rs_listing() {
  ( cd "$1" && find src tests -name '*.rs' -type f 2>/dev/null | sort | xargs -r sha256sum )
}

# Every bin target's source file for the crates in scope, as
# `<crate>/src/<file>` paths, relative to whichever crate root the crate sits
# under rather than to a named one. Empty output is a legitimate answer, because
# a family of pure libraries declares no binary. A non-zero return is not, and
# means the caller could not be told which files are binaries at all.
#
# Asked of cargo rather than guessed from a convention, because the convention
# is not the rule. G1 drops binary entry points from its coverage denominator
# and bounds how much may live in one, and both halves used to spell
# "binary entry point" as the literal path `src/main.rs`. A `[[bin]]` may
# declare any `path`. A crate declaring a second one at a non-conventional path
# would therefore be neither dropped nor bounded, sitting permanently in a
# denominator whose threshold is 100% and which no test suite can move.
#
# Asked of every workspace rather than of one, for a reason this function is
# its own best illustration of. Running `cargo metadata` once at `$REPO`
# answers for the root workspace's own trees and for nothing under a sibling
# workspace root (workspace_roots()). A crate living under such a root would
# go unseen by a fix that only widened `src/main.rs` handling, and its second
# binary would again go neither dropped nor bounded.
#
# The empty-output contract is what hid it. A family of pure libraries declaring
# no binary and a family cargo was never asked about produce the identical
# answer: rc=0 and nothing on stdout. Measured at the time of the fix, one
# affected family held multiple binaries well over G1's twelve-line shim budget,
# several of them by more than tenfold, and the gate saw none of them. So the
# `fail` that budget exists to trigger could not fire. Widening the question to
# every workspace is the whole of the fix. The contract itself is unchanged, and
# a genuinely bin-less family still answers exactly as it did.
family_bin_sources() {
  local root meta emitted names found=0 out=''
  names="$( family_crates | tr '\n' ' ' )"
  # A root that cannot be read at all is fatal, and deliberately so. That is the
  # "could not be told which files are binaries" case the header names, and it
  # has to stay distinguishable from the family that legitimately has none.
  # Asking every root and silently accepting that one of them answered nothing
  # would rebuild the exact blindness this loop replaced.
  while IFS= read -r root; do
    meta="$( cd "$root" && cargo metadata --no-deps --format-version 1 2>/dev/null )" || return 1
    [ -n "$meta" ] || return 1
    found=1
    # Keyed `<crate>/<path-within-crate>`, computed from the package's own
    # manifest directory instead of by stripping a list of known roots. That
    # spelling is what g1_coverage.sh's per_file join uses, and deriving it from
    # cargo's own answer makes it survive a relocation. A crate moving from one
    # crate root to another changes its manifest_path and its src_path together,
    # and the segment between them, the key, does not move.
    emitted="$( printf '%s' "$meta" | python3 -c '
import json, os, sys
want = set( sys.argv[ 1 : ] )

for package in json.load( sys.stdin )[ "packages" ]:
  if package[ "name" ] not in want:
    continue
  crate = os.path.dirname( package[ "manifest_path" ] )
  for target in package[ "targets" ]:
    if "bin" in target[ "kind" ]:
      print( package[ "name" ] + "/" + os.path.relpath( target[ "src_path" ], crate ) )
' $names )" || return 1
    # Only non-empty output is accumulated, and nothing downstream filters, so
    # this is the sole guard against a blank line reaching a caller. A result of
    # one blank line is not an empty result to a caller that mapfiles it into an
    # array. It is a one-element array holding the empty string, which
    # g1_coverage.sh would then hand to `crate_dir`.
    if [ -n "$emitted" ]; then out="$out$emitted"$'\n'; fi
  done < <( workspace_roots )
  [ "$found" -eq 1 ] || return 1
  # A family with zero `[[bin]]` targets is a legitimate result, not a failure,
  # and must stay distinguishable from a metadata failure by a caller's
  # `|| fail`. `sort` alone exits 0 on empty input, so no `|| true` guard is
  # needed here. One would only be needed if something that exits non-zero on
  # selecting nothing, such as a `grep`, were reintroduced into this pipeline.
  # Prefer not reintroducing one. The key is built correctly upstream now, so
  # there is nothing left to filter out.
  printf '%s' "$out" | sort -u
}

# What the current run is grading, for the gate's own message.
scope_label() {
  [ -n "${GATE_STAGE:-}" ] && printf ' [%s]' "$GATE_STAGE"
}

# Load a stage's crate and feature scope from the declaration. Exported so the
# gates a stage run invokes inherit it.
#
# Finding(load_stage_last_field_keeps_a_trailing_cr_on_crlf_input): same class
# of gap as `decl_lines` above, independently, since this reads `stages.txt` via
# `awk` rather than `grep -v`. Awk's default field splitting does not treat `\r`
# as a separator. So on a CRLF `stages.txt` the last field of the matched line,
# `GATE_FEATURES`'s value, would carry a trailing `\r` that `tr ',' '\n'` in
# `feature_ids()` then leaves glued to the last feature id. No live trigger:
# `stages.txt` is confirmed LF-only, same check as `decl_lines`.
load_stage() {
  local want="$1" decl="$DECL/stages.txt" line
  [ -f "$decl" ] || { echo "no stage declaration at ${decl#"$REPO"/}" >&2; return 1; }
  line="$( grep -E "^${want}[[:space:]]" "$decl" )" || {
    echo "unknown stage '${want}' — declared: $( grep -oE '^S[0-9]+' "$decl" | tr '\n' ' ' )" >&2
    return 1
  }
  export GATE_STAGE="$want"
  export GATE_CRATES="$( awk '{print $2}' <<<"$line" )"
  export GATE_FEATURES="$( awk '{print $3}' <<<"$line" )"
}

pass() { echo "REACHED    $GATE$( scope_label ) — $1"; exit 0; }
fail() { echo "NOT REACHED $GATE$( scope_label ) — $1"; exit 1; }

# ---- corpus gates (G14-G17, G20-G21) ----------------------------------------

# Where the corpus gates read their thresholds and vocabulary from.
#
# The control fixtures get their own, with the floors lowered and the tier
# vocabulary copied verbatim. A fixture built to 13/26/52 would be a fifth
# complete crate to maintain and every one of its findings would be about
# nothing, whereas a shortened tier list would let a fixture pass G17 for a
# reason no crate in the family could.
corpus_standard() {
  if [ -n "${GATE_CONTROL:-}" ]; then
    echo "$DECL/corpus_control/standard.txt"
  else
    echo "$DECL/corpus_standard.txt"
  fi
}

# The `docs/` directory of every crate in scope, or, under GATE_CONTROL, the
# control fixture instead.
#
# The control corpus is part of these gates. Each one passes by finding nothing,
# and the repository's conventions rulebook names that shape directly under
# *Documentation : Executable Recipes*: an empty result means
# "the property holds" and "the command is broken" equally well. Every way one
# of these checkers rots expresses itself as silence: a renamed definition, a
# changed table shape, a moved directory. So a corpus gate is only trustworthy
# in pairs, and `run_all.sh --control` is the other half. Under control, a gate
# sees only the fixtures named for it, `g16_*` for G16, and not any other
# gate's. That narrowing is the difference between proving that *something*
# fires and proving that *this* gate's own detector fires. The fixtures make the
# distinction real, not pedantic. Deleting a definition for G14 also breaks five
# links G16 would report, and G17's hybrid tier is also a disagreement between
# two tables that G16 would report. Pointed at every fixture at once, G16
# reports a defect whichever of its three assertions is broken; pointed at its
# own, it reports the one it was built to see. The same argument applies once
# more one level down, between the fixtures of a single gate. That is why they
# are graded one at a time below.
corpus_docs() {
  local c
  # Fix(g_corpus_docs_control_empty_glob_misreported_as_missing_crate_docs)
  # Root cause: with no corpus_control/<gate>_* directory declared for a family,
  #   bash's default (non-nullglob) leaves the for-loop's glob unexpanded. `c`
  #   binds to the literal pattern string once, its `[ -d "$c/docs" ]` test
  #   fails, and the bare `return` below then propagates that failed test's own
  #   status (1) with zero stderr output. The caller, run_corpus_checker()'s
  #   `corpus_out="$( corpus_docs )" || fail "corpus scope silently narrowed — a
  #   family crate carries no docs/ (see stderr above)"`, then reports a cause
  #   that never happened. No family crate was ever inspected on this branch at
  #   all, and it points at stderr output that was never written. Explicit
  #   `found` tracking plus a stderr diagnostic mirrors the non-control branch's
  #   own `skipped`-array idiom twelve lines below, which already gets this
  #   right for its own failure reason.
  # Pitfall: zero live trigger, measured directly, not assumed. No family under
  #   declared/ lists g14, g15, g16, g17, g20 or g21 in its gates.txt any more, so
  #   this branch runs only when those gates are named under `--control`. `ring`
  #   declares a corpus_control/ fixture directory for every one of the six, so
  #   this branch's glob always matches on every path reachable today. Verified behavior-identical for that live case (and
  #   for a synthetic ungrateful gate) by running both the old and new loop
  #   bodies side by side against the real declared/ring/corpus_control/ tree:
  #   identical stdout and return code for G14/G15/G16/G17/G20/G21, and the new
  #   stderr message fires correctly only for a gate with no fixtures at all.
  if [ -n "${GATE_CONTROL:-}" ]; then
    local found=0
    for c in "$DECL/corpus_control/${GATE,,}"_*/ ; do
      [ -d "$c/docs" ] || continue
      found=1
      printf '%s\n' "$c/docs"
    done
    if [ "$found" -eq 0 ]; then
      printf 'corpus_docs: no corpus_control/%s_* fixture directories declared under %s\n' \
        "${GATE,,}" "${DECL#"$REPO"/}" >&2
      return 1
    fi
    return 0
  fi
  # A crate declared into the family but carrying no docs/ must not vanish with
  # no signal. That is exactly the silence the pairing with `--control` exists
  # to distrust (see the comment above this function), and nothing upstream of a
  # checker checks it. Named and failed here, not merely dropped.
  local skipped=() d
  while read -r c; do
    if d="$( crate_dir "$c" )" && [ -d "$d/docs" ]; then
      printf '%s\n' "$d/docs"
    else
      skipped+=( "$c" )
    fi
  done < <( family_crates )
  if [ ${#skipped[@]} -gt 0 ]; then
    printf 'corpus_docs: %d crate(s) in scope carry no docs/, so no gate can grade them: %s\n' \
      "${#skipped[@]}" "${skipped[*]}" >&2
    return 1
  fi
}

# Split a checker's streams so an abort can be told from a verdict.
#
# A checker that crashed and a checker that found problems both exit 1, so the
# exit code alone cannot separate them, and that difference is the whole
# verdict. Measured, not assumed: G21 died on crate 13 of 34 with an unhandled
# `TypeError` (`recipes.blocks()` yields `command = None` for a probe recording;
# `addresses()` predated that shape), and the gate still printed
# `NOT REACHED G21 — ... across 33 crate(s)`. It had opened 12. The 14 problems
# it named were real; the 21 crates it never read were not covered, and nothing
# in the output said so. `g9_lint` discarding cargo's stderr and reporting a
# build failure as a scope gap is the same shape one gate over.
#
# The streams are kept apart instead of grepping the merged text, and that is
# not fastidiousness. No corpus checker writes to stderr, because
# `corpus_lib.report` prints findings to stdout. But `recipes.py` merges a
# *subprocess's* stderr into its own stdout finding text, so a corpus recipe
# whose command panics legitimately puts the word `Traceback` in a finding. On
# the merged stream that reads as a broken gate; on stderr alone it cannot.
run_checker_once() {
  local script="$1"; shift
  errf="$( mktemp )"
  # Fix(g_corpus_checker_errf_leak_on_signal)
  # Root cause: `errf` held a checker's stderr across a `python3` subprocess
  #   call with no trap at all. A SIGINT/TERM/HUP arriving while that subprocess
  #   ran (a real Ctrl-C during any of G14/G15/G16/G17/G20/G21, the six
  #   run_corpus_checker() callers that route through here) left the temp file
  #   behind with nothing to remove it, since the plain `rm -f` below is never
  #   reached once the shell dies to the signal.
  # Pitfall (x2, both confirmed against the live corpus): (1) EXIT-only,
  #   matching this file's other mktemp sites (g2_docs.sh, g7_determinism.sh,
  #   g9_lint.sh), not combining INT/TERM/HUP. Empirically, a combined trap that
  #   never calls `exit` in its body does NOT terminate the script on signal
  #   death. It only runs once and lets execution continue past the interruption
  #   (see g12_mutation.sh's own Fix comment for the same hazard hit head-on).
  #   EXIT alone already fires correctly because no competing handler overrides
  #   the signal's default terminating disposition. (2) `errf` was originally
  #   declared `local` here, alongside `script`, which stays local because only
  #   `errf` is named by the trap. But the trap referencing it is process-global
  #   and fires only when the WHOLE SCRIPT exits, almost always well after this
  #   function has already returned to its caller (`run_corpus_checker`, then
  #   the gate script's own `pass`/`fail`). By then the local binding was gone.
  #   Every gate script runs under `set -uo pipefail`, so expanding the
  #   now-unset "$errf" inside the trap body is itself a hard error that aborts
  #   the `rm -f` before it runs. Reproduced live: every family where
  #   `run_checker_once` executes (`GATE_FAMILY=ring` for a genuine PASS, any
  #   family with no corpus for a checker-side abort) printed
  #   `common.sh: line 1: errf: unbound variable` immediately after its own
  #   REACHED/NOT REACHED verdict line. Fixed the same way as
  #   g19_measured_columns.sh's identical `log` regression (see that file's own
  #   Fix comment): dropped `local` so `errf` stays a valid, if stale, reference
  #   for any later EXIT. Safe under `run_checker_once`'s own per-crate loop
  #   caller too. Each iteration's manual `rm -f "$errf"` below already clears
  #   that iteration's file before the next iteration reassigns the global, so
  #   only the single most-recently-live value is ever what the trap needs to
  #   see.
  trap 'rm -f -- "$errf"' EXIT
  CHECKER_OUT="$( cd "$( dirname "${BASH_SOURCE[0]}" )/corpus" \
    && python3 "$script" "$@" 2>"$errf" )"
  CHECKER_RC=$?
  CHECKER_ERR="$( cat "$errf" )"
  rm -f "$errf"
}

# True when the checker did not run to completion: a Python traceback on
# stderr, or an exit code outside { 0 = clean, 1 = findings }.
checker_crashed() {
  local rc="$1" err="$2"
  case "$err" in
    *'Traceback (most recent call last):'* ) return 0 ;;
  esac
  [ "$rc" -le 1 ] && return 1
  return 0
}

# Run one corpus checker over everything in scope and translate its exit code.
# Under GATE_CONTROL the polarity inverts: a checker that finds nothing wrong
# with a deliberately broken fixture is the failure being reported.
run_corpus_checker() {
  local script="$1" what="$2" out rc
  shift 2
  local dirs=() corpus_out
  # Command, not process, substitution. corpus_docs()'s exit code has to reach
  # this shell to be actionable, and a process-substituted subshell's does not.
  corpus_out="$( corpus_docs )" \
    || fail "corpus scope silently narrowed — a family crate carries no docs/ (see stderr above)"
  while IFS= read -r d; do [ -n "$d" ] && dirs+=( "$d" ); done <<< "$corpus_out"
  [ ${#dirs[@]} -gt 0 ] || fail "no docs/ directory in scope — nothing to grade"

  # Under control each fixture is graded alone, against what its own name
  # declares. Grading them together only ever asked whether *something* fired,
  # which a gate holding more than one fixture can answer from the defect it can
  # already see. So a second fixture could never expose a blind spot the first
  # one masked. That is not hypothetical. G21 could not see the multi-window
  # form `sed -n '1,2p;5,6p'`, and carried 148 live occurrences of it across 15
  # crates while `g21_line_addressed` kept the control green on its behalf. The
  # `_clean` fixtures invert the expectation and must report nothing, which the
  # aggregate did not check at all. `g20_clean` was documented as the proof G20
  # can pass and was never graded.
  if [ -n "${GATE_CONTROL:-}" ]; then
    local d name saw_clean=0
    for d in "${dirs[@]}"; do
      name="$( basename "$( dirname "$d" )" )"
      run_checker_once "$script" "$@" "$d"
      out="$CHECKER_OUT"; rc="$CHECKER_RC"
      echo "$out" | sed 's/^/    /'
      [ -n "$CHECKER_ERR" ] && echo "$CHECKER_ERR" | sed 's/^/    /'
      # Before the fixture's own verdict is read: a crash exits 1 exactly like
      # a seeded defect being caught, so an aborted checker would *prove* this
      # gate can see the defect it never reached.
      checker_crashed "$rc" "$CHECKER_ERR" \
        && fail "control: $script aborted on $name (exit $rc) — this fixture graded nothing"
      case "$name" in
        *_clean )
          saw_clean=1
          [ "$rc" -eq 0 ] \
            || fail "control: $name is the must-pass fixture and this gate reported a problem in it" ;;
        * )
          [ "$rc" -ne 0 ] \
            || fail "control: $name reported clean — this gate cannot see the defect seeded for it" ;;
      esac
    done
    # Every defect fixture reporting dirty is only half the proof. A checker
    # that flags everything, clean input included, would pass this loop too.
    # Without at least one `*_clean` fixture in scope there is no case above
    # that can ever run the `-eq 0` branch, so a false-positive-only checker is
    # indistinguishable from a correct one. That is the exact blindness the
    # comment above this function names `g20_clean` as the historical proof of.
    [ "$saw_clean" -eq 1 ] \
      || fail "control: no *_clean fixture in scope — this gate's control run never proved it accepts good input"
    pass "control: each of ${#dirs[@]} fixture crate(s) behaved as its name declares"
  fi

  run_checker_once "$script" "$@" "${dirs[@]}"
  out="$CHECKER_OUT"; rc="$CHECKER_RC"

  if [ "$rc" -ne 0 ]; then
    echo "$out" | sed 's/^/    /'
    [ -n "$CHECKER_ERR" ] && echo "$CHECKER_ERR" | sed 's/^/    /'
    checker_crashed "$rc" "$CHECKER_ERR" \
      && fail "$script aborted mid-scan (exit $rc) — no verdict about ${#dirs[@]} crate(s) was earned"
    fail "$what across ${#dirs[@]} crate(s)"
  fi
  pass "$what across ${#dirs[@]} crate(s)"
}

# The non-vacuity check every gate shares. A declaration is only worth grading
# against if the crates it names are there. Called first by every gate, so no
# gate can report REACHED about a family that does not exist.
assert_declared_crates_exist() {
  local c missing=() n=0
  [ -d "$DECL" ] || fail "no declaration directory for family '${GATE_FAMILY:-<unset>}' at ${DECL#"$REPO"/}"
  while read -r c; do
    n=$(( n + 1 ))
    crate_dir "$c" >/dev/null || missing+=( "$c" )
  done < <( family_crates )
  [ "$n" -gt 0 ] || fail "family '$GATE_FAMILY' declares no crates in scope — nothing to grade"
  [ ${#missing[@]} -eq 0 ] || fail "${#missing[@]}/${n} declared crate(s) absent from the tree: ${missing[*]}"
}
