#!/usr/bin/env python3
"""PostToolUse hook: formats a `.rs` file the agent just wrote, the way `verb/fmt` would.

Only when the file was already formatted at `HEAD` (or is new): reformatting a file that was not
would put every untouched line of it into the edit's diff. Skips files `cargo fmt` never reaches
and that must stay byte-exact: trybuild inputs (`tests/ui/`, their `.stderr` pins line numbers)
and gate fixtures (`bench_harness/gate/`). A missing nightly or a file that does not parse yet is
not an error here — `verb/fmt check::1` is the gate.
"""

import json
import subprocess
import sys
from pathlib import Path

SKIP = ("/tests/ui/", "/bench_harness/gate/", "/target/")


def run(args, cwd, stdin=None):
  """Runs a command, returning `(returncode, stdout)`, or `None` if it could not start."""
  try:
    done = subprocess.run(
      args, cwd=cwd, input=stdin, capture_output=True, text=True, timeout=30
    )
  except (OSError, subprocess.SubprocessError):
    return None
  return done.returncode, done.stdout


def clean_at_head(path, root, config):
  """Whether `path` at `HEAD` is already rustfmt-clean; `True` for a file `HEAD` lacks."""
  rel = path.relative_to(root).as_posix()
  shown = run(["git", "show", f"HEAD:{rel}"], root)
  if shown is None or shown[0] != 0:
    return True
  formatted = run(
    ["rustfmt", "+nightly", "--emit", "stdout", "--config-path", str(config)], root, shown[1]
  )
  return formatted is not None and formatted[0] == 0 and formatted[1] == shown[1]


def main():
  try:
    payload = json.load(sys.stdin)
  except ValueError:
    return 0

  raw = (payload.get("tool_input") or {}).get("file_path")
  if not raw or not raw.endswith(".rs") or any(s in raw for s in SKIP):
    return 0
  path = Path(raw).resolve()
  if not path.is_file():
    return 0

  top = run(["git", "rev-parse", "--show-toplevel"], path.parent)
  if top is None or top[0] != 0:
    return 0
  root = Path(top[1].strip())
  config = root / "rustfmt.toml"

  if clean_at_head(path, root, config):
    run(
      [
        "rustfmt", "+nightly", "--unstable-features", "--skip-children",
        "--config-path", str(config), str(path),
      ],
      root,
    )
  return 0


if __name__ == "__main__":
  sys.exit(main())
