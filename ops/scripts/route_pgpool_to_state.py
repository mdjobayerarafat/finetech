#!/usr/bin/env python3
"""Convert handlers that take `pool: web::Data<PgPool>` to use
`state: web::Data<AppState>` + `state.pool`, then add the `crate::state::AppState`
import if missing.

Only touches the routes/ directory (one-shot bulk fix).
"""
import re
import sys
from pathlib import Path

# Match `pool: web::Data<PgPool>` in function signatures and convert.
POOL_SIG = re.compile(r"\bpool\s*:\s*web::Data<PgPool>")
POOL_DEREF = re.compile(r"&\*\*pool")
USE_CRATE_STATE = re.compile(r"^use\s+crate::state::AppState\s*;\s*$", re.MULTILINE)


def convert(path: Path) -> tuple[bool, int]:
    src = path.read_text()
    changed = False

    # 1. Replace the signature.
    new, n = POOL_SIG.subn("state: web::Data<AppState>", src)
    if n:
        src = new
        changed = True

    # 2. Replace usages `&**pool` -> `&state.pool`.
    new, n = POOL_DEREF.subn("&state.pool", src)
    if n:
        src = new
        changed = True

    # 3. Make sure `use crate::state::AppState;` is present.
    if changed and not USE_CRATE_STATE.search(src):
        # Insert after the first `use crate::...` line that uses `ctx` or
        # any `use crate::*;` line near the top. Fallback: top of file.
        m = re.search(r"^(use\s+crate::[^\n]+;\s*\n)+", src, re.MULTILINE)
        if m:
            insert_at = m.end()
            src = src[:insert_at] + "use crate::state::AppState;\n" + src[insert_at:]
        else:
            src = "use crate::state::AppState;\n" + src

    if changed:
        path.write_text(src)
    return changed, n


def main(argv: list[str]) -> int:
    total = 0
    for arg in argv[1:]:
        p = Path(arg)
        if not p.is_file():
            continue
        c, n = convert(p)
        if c:
            print(f"{p}: rewrote {n} pool→state")
            total += 1
    print(f"---\nfiles changed: {total}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
