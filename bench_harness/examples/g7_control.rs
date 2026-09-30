//! Control fixture for G7 — one binary that can produce each defect G7 claims to detect.
//!
//! G7's fourth comparison is a *must-differ* check: a smoke binary's output has
//! to move when its input moves before byte-identity across runs, opt-levels and
//! targets counts as evidence. A must-differ check that is never observed
//! differing is exactly the unfalsifiable shape it was added to close, one level
//! up. So this fixture exists to be failed, deliberately, on demand.
//!
//! Selected by `--mode`, seeded by `--seed`. One mode per row of task 153's Test
//! Matrix, and each mode breaks exactly one property so the gate's message can be
//! checked against a known cause rather than against whatever it happens to say:
//!
//! | mode      | varies with seed | stable run to run | debug == release | native == musl |
//! |-----------|------------------|-------------------|------------------|----------------|
//! | `constant`| no               | yes               | yes              | yes            |
//! | `varying` | yes              | yes               | yes              | yes            |
//! | `nondet`  | yes              | **no**            | yes              | yes            |
//! | `optlevel`| yes              | yes               | **no**           | yes            |
//! | `xtarget` | yes              | yes               | yes              | **no**         |
//!
//! `varying` is the only row that reaches. Run it through the gate with
//! `G7_CONTROL=<mode> bash ring/bench_harness/gate/g7_determinism.sh`.

use std::time::{SystemTime, UNIX_EPOCH};

/// Which property this run breaks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Prints the same bytes whatever the seed — the vacuity G7 must now reject.
    Constant,
    /// Seed-dependent and stable on every other axis. The one reaching mode.
    Varying,
    /// Seed-dependent, but a fresh reading of the clock moves it run to run.
    Nondet,
    /// Seed-dependent, but `debug_assertions` puts a different line in each profile.
    OptLevel,
    /// Seed-dependent, but the target's C environment puts a different line in each build.
    XTarget,
}

impl Mode {
    /// Parses the `--mode=` value, naming the accepted set on anything else.
    fn parse(text: &str) -> Result<Self, String> {
        match text {
            "constant" => Ok(Self::Constant),
            "varying" => Ok(Self::Varying),
            "nondet" => Ok(Self::Nondet),
            "optlevel" => Ok(Self::OptLevel),
            "xtarget" => Ok(Self::XTarget),
            other => Err(format!(
                "unknown mode {other:?} — expected constant, varying, nondet, optlevel or xtarget"
            )),
        }
    }
}

/// A seed-derived digest. Deliberately a plain integer mix rather than a hash
/// crate: this fixture must not acquire a dependency whose own determinism the
/// gate would then be measuring instead of its own.
fn digest(seed: u64) -> u64 {
    let mut value = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 31;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value
}

/// Reads `--mode=` and `--seed=` out of the command line.
fn parse_args(args: &[String]) -> Result<(Mode, u64), String> {
    let mut mode = None;
    let mut seed = 0_u64;
    for arg in args {
        if let Some(value) = arg.strip_prefix("--mode=") {
            mode = Some(Mode::parse(value)?);
        } else if let Some(value) = arg.strip_prefix("--seed=") {
            seed = value.parse::<u64>().map_err(|_| format!("seed {value:?} is not a number"))?;
        } else {
            return Err(format!("unexpected argument {arg:?} — expected --mode= and --seed="));
        }
    }
    mode.map(|m| (m, seed)).ok_or_else(|| "no --mode= given".to_string())
}

/// The report body. Three lines in every mode, so a diff between two modes is
/// about content rather than about length.
fn report(mode: Mode, seed: u64) -> String {
    let head = "g7 control";
    let body = match mode {
        Mode::Constant => "fixed 0000000000000000".to_string(),
        _ => format!("seeded {:016x}", digest(seed)),
    };
    let tail = match mode {
        Mode::Nondet => {
            // A fresh clock reading, which is what makes two runs at the same seed
            // disagree. Nanoseconds rather than seconds so the gate's back-to-back
            // runs cannot land in the same tick and report this mode as stable.
            let now =
                SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
            format!("clock {now:09}")
        },
        Mode::OptLevel => {
            // Resolved at compile time, so the two profiles genuinely emit different
            // bytes rather than branching on something the optimiser could fold away.
            if cfg!(debug_assertions) {
                "profile debug".to_string()
            } else {
                "profile release".to_string()
            }
        },
        Mode::XTarget => {
            if cfg!(target_env = "musl") {
                "env musl".to_string()
            } else {
                "env glibc".to_string()
            }
        },
        _ => "stable".to_string(),
    };
    format!("{head}\n{body}\n{tail}\n")
}

fn main() -> std::process::ExitCode {
    // Fix(g7_control_argv_invalid_utf8_panics_before_parse_args): CLI argv
    // reaches `std::env::args()`, whose contract is to panic during iteration on
    // the first argument that is not valid Unicode -- so a non-UTF-8 argument
    // (e.g. from a calling script or a stray glob expansion) aborts the process
    // before `parse_args`'s own `Result`-based refusal, which already handles
    // every other kind of bad argument gracefully, ever runs. Fixed by
    // collecting through `args_os()` and converting each argument with
    // `to_string_lossy()`, so a non-UTF-8 argument becomes a
    // replacement-character-bearing `String` that reaches `parse_args` instead
    // of crashing first.
    // Pitfall: a parser returning `Result<_, String>` for every bad value it can
    // see is not automatically panic-free -- `std::env::args()` itself can
    // panic before `parse_args` ever runs, and no in-process test built from a
    // `Vec<String>` can construct the input that reaches it.
    let args: Vec<String> =
        std::env::args_os().skip(1).map(|arg| arg.to_string_lossy().into_owned()).collect();
    match parse_args(&args) {
        Ok((mode, seed)) => {
            print!("{}", report(mode, seed));
            std::process::ExitCode::SUCCESS
        },
        Err(error) => {
            eprintln!("g7_control: {error}");
            std::process::ExitCode::FAILURE
        },
    }
}
