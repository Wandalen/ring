#!/usr/bin/env bash
# G7. Every declared smoke binary's stdout MOVES when its declared input moves,
# and is byte-identical across two runs, across debug and release, and across
# two instruction sets, AND at least one such binary exists with non-empty
# output.
#
# Four comparisons, and the first one is the odd one out. Three assert sameness,
# one asserts difference. The must-differ comparison came last and runs first,
# because the other three are each true of a program that prints a constant. A
# binary proving nothing at all used to collect the strongest verdict this suite
# issues. It is checked against the family's own documented input knob
# (`smoke.txt`), never a guessed one. Its failure is worded so it cannot be read
# as a determinism failure, because those are opposite defects.
#
# This is the ring family's top invariant turned into a gate. Determinism ranks
# above accuracy on the ground that an inaccurate propagator
# that is bit-reproducible produces a world that is consistently wrong, while
# an accurate one that is not reproducible produces divergence between clients
# and between a run and its own replay.
#
# The debug/release half is the one that bites. Two runs of the same binary
# agree almost by construction; the same arithmetic compiled at a different
# optimisation level does not, because the optimiser is free to contract a
# multiply-add into an FMA, reassociate a sum, or keep an intermediate in a
# wider register. Every one of those changes the low bits, and none of them
# changes the source.
#
# Non-vacuity: a family with no smoke binary, or a binary that prints nothing,
# would satisfy "all outputs agree" trivially. Both are failures here.
set -uo pipefail
GATE=G7
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist
command -v jq >/dev/null || fail "jq is not on PATH — this gate asks cargo whether a package declares a bin target"

# The second target. Hard-required, not skipped-when-absent, on the same
# reasoning as the jq check above. A gate that drops its own strongest claim
# without a word reports "reached" for something it did not check, which is
# worse than not running. Added at stage S12, whose whole goal is this
# comparison.
XTARGET=x86_64-unknown-linux-musl
XRUN=qemu-x86_64-static
XFLAGS="-C link-self-contained=yes -C linker=rust-lld"
rustup target list --installed 2>/dev/null | grep -qx "$XTARGET" \
  || fail "rust target $XTARGET is not installed — 'rustup target add $XTARGET'"
command -v "$XRUN" >/dev/null \
  || fail "$XRUN is not on PATH — needed to execute the $XTARGET build on this host"

# The control switch. `G7_CONTROL=<mode>` grades this crate's own
# `examples/g7_control.rs` instead of the family's smoke binaries, with the mode
# selecting exactly one defect. It exists because the fourth comparison below is
# a *must-differ* check. A must-differ check nobody has watched differ is the
# same unfalsifiable shape that comparison was added to close, one level up.
#
# Deliberately not `--control`/`GATE_CONTROL`. That switch inverts polarity, so
# a corpus gate REACHES by finding its seeded defect. Here the control must be
# reported NOT REACHED. Two opposite meanings on one switch is how a control
# stops being readable.
CONTROL_PKG=bench_harness
CONTROL_EX=g7_control

# What to build and run for one smoke entry. A family entry is a package; the
# control is an example of this crate. Every build and run below goes through
# this, so the two cases diverge in exactly one place.
smoke_sel() {
  if [ -n "${G7_CONTROL:-}" ]; then
    printf '%s\n' -p "$CONTROL_PKG" --example "$CONTROL_EX"
  else
    printf '%s\n' -p "$1"
  fi
}

# WHERE to run the above from. Branches on `G7_CONTROL` exactly once, the same
# single place `smoke_sel` does, so the control and family cases cannot drift
# into disagreeing about which package is being graded.
#
# Every cargo invocation in this file used to run at `$REPO`. That is right for
# workspaces nested directly under the repo root and wrong for a sibling
# family's own separately-rooted workspace, which `cargo metadata` at `$REPO`
# cannot see at all. A declared smoke binary can live in exactly such a
# workspace. The failure was silent in the worst available way. The bin-target
# query at `$REPO` returned empty for a package cargo had never heard of, and
# the gate reported "declares no bin target". That is a statement about the
# crate's own manifest, for a crate that declares exactly one.
#
# Falls back to `$REPO` when the lookup fails instead of aborting. A name this
# script cannot resolve still reaches the build below and fails there, with
# cargo's own diagnostic, instead of being reclassified here without a word.
smoke_root() {
  local pkg root
  if [ -n "${G7_CONTROL:-}" ]; then pkg="$CONTROL_PKG"; else pkg="$1"; fi
  root="$( crate_workspace_root "$pkg" 2>/dev/null )" || root=""
  [ -n "$root" ] || root="$REPO"
  printf '%s\n' "$root"
}

# Arguments the identity comparisons pass. Family binaries are run bare, exactly
# as before this gate grew a fourth comparison; the control needs its mode named
# on every invocation or it has nothing to be.
smoke_base_args() {
  [ -n "${G7_CONTROL:-}" ] && printf '%s\n' "--mode=${G7_CONTROL}" "--seed=1"
  return 0
}

# The declared input knob for one smoke binary, two invocations whose outputs
# must differ. Read from a `# knob:` directive in the family's own `smoke.txt`:
#
#   # knob: <bin>  <invocation A>  ||  <invocation B>
#
# A comment, so `family_smoke` still yields one bare package name per line and
# every existing reader of that file is untouched. Each invocation is a
# whitespace-separated list of arguments, `VAR=value` assignments, or both. That
# lets one syntax cover `demo_binary --beat=S1` and
# `MODE=alt smoke_other_binary` without the gate knowing that either binary
# takes its input a particular way.
smoke_knob() {
  grep -E "^[[:space:]]*#[[:space:]]*knob:[[:space:]]+$1([[:space:]]|\$)" "$DECL/smoke.txt" 2>/dev/null \
    | head -1 \
    | sed -E "s/^[[:space:]]*#[[:space:]]*knob:[[:space:]]+$1[[:space:]]+//"
}

# Runs one knob arm into a file. Leading `VAR=value` words become environment for
# that run; everything from the first non-assignment onward is arguments.
#
# The caller passes the owning workspace in, and this function does not look it
# up. The caller has already resolved it once for this binary, and `smoke_root`
# walks the tree to answer.
run_arm() {
  local out="$1" bin="$2" ws="$3"
  shift 3
  local envs=() argv=() w sel
  for w in "$@"; do
    if [ ${#argv[@]} -eq 0 ] && [[ "$w" == *=* && "$w" != -* ]]; then
      envs+=( "$w" )
    else
      argv+=( "$w" )
    fi
  done
  mapfile -t sel < <( smoke_sel "$bin" )
  ( cd "$ws" && env ${envs[@]+"${envs[@]}"} cargo run -q "${sel[@]}" -- ${argv[@]+"${argv[@]}"} ) \
    >"$out" 2>/dev/null
}

if [ -n "${G7_CONTROL:-}" ]; then
  smoke=( "$CONTROL_EX" )
  echo "── control: grading examples/${CONTROL_EX}.rs in mode '${G7_CONTROL}' — NOT REACHED is the expected verdict for every mode but 'varying'"
else
  mapfile -t smoke < <( family_smoke )
  [ ${#smoke[@]} -gt 0 ] || fail "family '$GATE_FAMILY' declares no smoke binary — nothing to diff"
fi

# Under a stage run, only grade the smoke binaries that stage owns. A stage
# whose crates include none of them has no determinism claim of its own to make
# and the gate is not applicable rather than failed.
if [ -n "${GATE_CRATES:-}" ]; then
  in_scope=()
  while read -r c; do
    printf '%s\n' "${smoke[@]}" | grep -qx "$c" && in_scope+=( "$c" )
  done < <( family_crates )
  if [ ${#in_scope[@]} -eq 0 ]; then
    pass "no smoke binary in this stage's crates — determinism graded by the stage that owns one"
  fi
  smoke=( "${in_scope[@]}" )
fi

work="$( mktemp -d )"
trap 'rm -rf "$work"' EXIT

differ=() empty=() unbuilt=() crashed=() novary=() noknob=()
for bin in "${smoke[@]}"; do
  mapfile -t sel < <( smoke_sel "$bin" )
  mapfile -t base < <( smoke_base_args )
  # Resolved once per binary and reused by every cargo invocation below. The
  # build pair, the bin-target query, the four runs, and the cross-target build
  # must all address the same workspace, or a crate found by one is invisible to
  # the next.
  ws="$( smoke_root "$bin" )"
  # These three build failures print their captured log before joining
  # `unbuilt`, matching g2_docs.sh's build/doctest checks. A compiler diagnostic
  # needs full detail. The run-crash branches below bound their snippet to 160
  # characters by design (line ~224), because they capture a live run's stderr,
  # not a build log.
  if ! ( cd "$ws" && cargo build "${sel[@]}" ) >"$work/build_dbg.log" 2>&1; then
    cat "$work/build_dbg.log"
    unbuilt+=( "$bin (debug build failed)" ); continue
  fi
  if ! ( cd "$ws" && cargo build "${sel[@]}" --release ) >"$work/build_rel.log" 2>&1; then
    cat "$work/build_rel.log"
    unbuilt+=( "$bin (release build failed)" ); continue
  fi

  # Three outcomes exit non-zero, and NEITHER the exit code nor stdout can tell
  # them apart:
  #
  #   * a package with no binary target: cargo's own error, exit 101, no stdout
  #   * a panic partway through the run: exit 101, and stdout is whatever the
  #     binary had flushed, which for a demo that renders its report at the end
  #     is nothing at all
  #   * a run that completed and graded itself NOT REACHED: exit 1, full stdout
  #
  # Only the first two are G7's business. A red beat is still a determinism
  # claim, and the verdict belongs to the stage's own reached-test.
  #
  # Before stage S6, all three were reported as "no runnable binary". The first
  # fix let stdout decide whether it ran and the exit code decide whether it
  # finished. Stage S6's manual plan then measured a panicking demo and found
  # stdout at zero bytes. So the two 101 cases were still indistinguishable and
  # the fix had not fixed anything.
  #
  # So the binary-target question is settled statically, before running, and the
  # exit code then means only what the binary itself chose it to mean.
  if [ -n "${G7_CONTROL:-}" ]; then
    targets="$CONTROL_EX"
  else
    targets="$( cd "$ws" && cargo metadata --no-deps --format-version 1 2>/dev/null \
      | jq -r --arg p "$bin" '.packages[] | select( .name == $p ) | .targets[] | select( .kind[] == "bin" ) | .name' )"
  fi
  # An empty answer has two causes, and this query cannot tell them apart.
  # Either the package really declares no `[[bin]]`, or it was not a member of
  # the workspace the query ran in. The message therefore reports the
  # observation ("no bin target visible from <ws>"), not the manifest claim it
  # used to assert. A failed lookup has no standing to describe a crate's own
  # Cargo.toml. Naming the workspace also makes the second cause diagnosable
  # from the verdict line alone.
  if [ -z "$targets" ]; then
    ws_rel="${ws#"$REPO"}"; ws_rel="${ws_rel#/}"; [ -n "$ws_rel" ] || ws_rel='repo root'
    unbuilt+=( "$bin (no bin target visible from $ws_rel)" ); continue
  fi

  # ── The fourth comparison, and the first one run ────────────────────────────
  #
  # Output must MOVE when input moves. Everything below this point is an
  # identity check, and every one of them is true of a program that prints a
  # constant. Without this the gate congratulates precisely the binary that
  # proves nothing. Run first, and failing it skips the rest. A binary whose
  # output ignores its input has no determinism claim worth the cross-target
  # build.
  #
  # Reported apart from a determinism failure because the two are opposite
  # defects. This one says the output ignores its input; that one says the output
  # tracks something the input does not name. A reader sent to fix the wrong one
  # of those has been actively misled, which is worse than an unreported defect.
  if [ -n "${G7_CONTROL:-}" ]; then
    knob="--mode=${G7_CONTROL} --seed=1 || --mode=${G7_CONTROL} --seed=2"
  else
    knob="$( smoke_knob "$bin" )"
  fi
  if [ -z "$knob" ] || [ "$knob" = "${knob#*||}" ]; then
    noknob+=( "$bin" ); continue
  fi
  # Finding(g7_knob_arm_unquoted_glob_expansion): `$arm_a`/`$arm_b` are expanded
  # unquoted at every use below (`run_arm ... $arm_a`, `run_arm ... $arm_b`, and
  # inside `echo $arm_a`/`echo $arm_b` on the novary message further down). That
  # is deliberate. Each arm is a whitespace-separated list of `VAR=value` words
  # and/or CLI flags that must split into several words for `run_arm`'s own
  # `"$@"` loop to see them as separate tokens (see `smoke_knob`'s header
  # comment above). What is not guarded is pathname expansion. If a declared
  # knob arm ever contained a `*`, `?`, or `[`, the unquoted expansion would
  # also be glob-matched against the process's own cwd before `run_arm` ever saw
  # it. That would replace the declared argument with whatever files happened to
  # match, without a word (or leave it untouched only if nothing matched). No
  # `set -f` is in effect anywhere in this file. No live trigger: confirmed
  # empirically that this repository currently declares zero `# knob:` lines at
  # all (`ring/smoke.txt` declares no smoke binaries). So there is nothing on
  # disk today for a glob metacharacter to hide in. The hazard is real but
  # dormant.
  arm_a="${knob%%||*}"
  arm_b="${knob##*||}"
  run_arm "$work/$bin.k1" "$bin" "$ws" $arm_a; s1=$?
  run_arm "$work/$bin.k2" "$bin" "$ws" $arm_b; s2=$?
  if [ $s1 -gt 1 ] || [ $s2 -gt 1 ]; then
    crashed+=( "$bin (exit $s1/$s2 under its declared input knob)" ); continue
  fi
  if [ ! -s "$work/$bin.k1" ] || [ ! -s "$work/$bin.k2" ]; then
    empty+=( "$bin (under its declared input knob)" ); continue
  fi
  if cmp -s "$work/$bin.k1" "$work/$bin.k2"; then
    novary+=( "$bin ($( echo $arm_a ) against $( echo $arm_b ))" ); continue
  fi

  ( cd "$ws" && cargo run -q "${sel[@]}" -- ${base[@]+"${base[@]}"} ) >"$work/$bin.a" 2>"$work/$bin.err"
  status=$?

  # Exit 0 reached everything, exit 1 ran everything and graded some of it red.
  # Both produced a whole report and are diffable. Anything else did not finish,
  # and an unfinished report diffs clean against another unfinished report.
  # Without this the gate would report byte-identical determinism for a binary
  # that died twice in the same place.
  if [ $status -gt 1 ]; then
    crashed+=( "$bin (exit $status: $( tr '\n' ' ' <"$work/$bin.err" | cut -c1-160 ))" )
    continue
  fi

  if [ ! -s "$work/$bin.a" ]; then
    empty+=( "$bin" ); continue
  fi

  ( cd "$ws" && cargo run -q "${sel[@]}" -- ${base[@]+"${base[@]}"} ) >"$work/$bin.b" 2>"$work/$bin.b.err"
  status_b=$?
  ( cd "$ws" && cargo run -q "${sel[@]}" --release -- ${base[@]+"${base[@]}"} ) >"$work/$bin.r" 2>"$work/$bin.r.err"
  status_r=$?

  # Fix(g7_b_and_r_runs_exit_status_never_checked)
  # Root cause: only the first debug run (.a, line ~216) and the cross-target
  #   run (.x, line ~257) ever read their own exit code. This second debug run
  #   (.b) and the release run (.r) piped stderr to /dev/null and never captured
  #   $?. A crash on either produced truncated or empty output that then reached
  #   only the `cmp` below, so it showed up as
  #   "output differs (run to run)"/"(debug to release)" instead of "crashed".
  #   That is the misattribution this gate's own header (lines 150-167) already
  #   explains at length: a reader sent to hunt a determinism bug when the real
  #   defect is a crash.
  # Pitfall: `crashed` is deliberately checked ahead of `differ` in the final
  #   report (lines ~270-283) so a crash is never shadowed by a lower-priority
  #   verdict. But that ordering only protects crashes this loop classifies as
  #   `crashed`. One folded into `differ` without a word bypasses the priority
  #   no matter how carefully the report order was designed.
  if [ $status_b -gt 1 ]; then
    crashed+=( "$bin (exit $status_b on the run-to-run comparison run: $( tr '\n' ' ' <"$work/$bin.b.err" | cut -c1-160 ))" )
    continue
  fi
  if [ $status_r -gt 1 ]; then
    crashed+=( "$bin (exit $status_r on the debug-to-release comparison run: $( tr '\n' ' ' <"$work/$bin.r.err" | cut -c1-160 ))" )
    continue
  fi

  cmp -s "$work/$bin.a" "$work/$bin.b" || differ+=( "$bin (run to run)" )
  cmp -s "$work/$bin.a" "$work/$bin.r" || differ+=( "$bin (debug to release)" )

  # And across a target. Everything above varies the optimiser while holding the
  # instruction set, the libm and the ABI fixed. This varies all three at once,
  # aarch64/glibc against x86-64/musl. That is the only arrangement in which the
  # determinism invariant is tested where it matters, not just locally true.
  #
  # Compared against the native *release* output so the optimisation level is
  # held constant and the target is the only thing that moved.
  target_bin="$( printf '%s\n' "$targets" | head -1 )"
  if ! ( cd "$ws" && RUSTFLAGS="$XFLAGS" cargo build -q "${sel[@]}" --release --target "$XTARGET" ) \
    >"$work/build_x.log" 2>&1; then
    cat "$work/build_x.log"
    unbuilt+=( "$bin ($XTARGET build failed)" ); continue
  fi
  # Where the artifact landed is cargo's answer to give, not something this
  # script can assemble from `$ws`. `CARGO_TARGET_DIR` relocates the whole tree
  # out of the workspace (which is how these gates run concurrently without two
  # cargos fighting over one lock), and the build above honoured it. Falling
  # back to `$ws/target` only covers the unset case.
  tdir="$( cd "$ws" && cargo metadata --no-deps --format-version 1 2>/dev/null | jq -r '.target_directory' )"
  [ -n "$tdir" ] && [ "$tdir" != null ] || tdir="$ws/target"
  # cargo puts an example one directory deeper than a bin.
  if [ -n "${G7_CONTROL:-}" ]; then
    xbin="$tdir/$XTARGET/release/examples/$target_bin"
  else
    xbin="$tdir/$XTARGET/release/$target_bin"
  fi
  # A path that resolved to nothing is a failure of this gate, not a binary that
  # crashed. Without this check it arrives as one, because `$XRUN` on a missing
  # file exits 125/127 and every exit above 1 is classified `crashed` below. The
  # gate would then send a reader to debug a determinism bug in a binary that
  # never ran.
  if [ ! -x "$xbin" ]; then
    unbuilt+=( "$bin ($XTARGET artifact not found at $xbin)" ); continue
  fi
  "$XRUN" "$xbin" ${base[@]+"${base[@]}"} >"$work/$bin.x" 2>/dev/null
  status=$?
  if [ $status -gt 1 ]; then
    crashed+=( "$bin (exit $status on $XTARGET)" ); continue
  fi
  cmp -s "$work/$bin.r" "$work/$bin.x" || differ+=( "$bin (aarch64/glibc to x86-64/musl)" )
done

if [ ${#unbuilt[@]} -gt 0 ]; then
  fail "${#unbuilt[@]} smoke binary target(s) do not build: ${unbuilt[*]}"
fi
if [ ${#empty[@]} -gt 0 ]; then
  fail "${#empty[@]} smoke binary/binaries print nothing — an empty run is not a determinism claim: ${empty[*]}"
fi
if [ ${#crashed[@]} -gt 0 ]; then
  fail "${#crashed[@]} smoke binary/binaries died partway through the run: ${crashed[*]}"
fi
# The two vacuity verdicts, reported before the determinism one and worded so
# they cannot be mistaken for it. "Does not vary" and "outputs differ" are
# opposite failures, and the fix for one is the defect in the other.
if [ ${#noknob[@]} -gt 0 ]; then
  fail "${#noknob[@]} smoke binary/binaries declare no input knob in smoke.txt, so output cannot be shown to vary with input — add a '# knob: <bin> <a> || <b>' line: ${noknob[*]}"
fi
if [ ${#novary[@]} -gt 0 ]; then
  fail "${#novary[@]} smoke binary/binaries produce identical output from different input — vacuous, not deterministic: ${novary[*]}"
fi
if [ ${#differ[@]} -gt 0 ]; then
  fail "${#differ[@]} output(s) differ: ${differ[*]}"
fi

lines="$( wc -l < "$work/${smoke[0]}.a" )"
pass "${#smoke[@]} smoke binary/binaries vary with their declared input, and are byte-identical run to run, debug to release, and aarch64/glibc to x86-64/musl (${lines} lines)"
