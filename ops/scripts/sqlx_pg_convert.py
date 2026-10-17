#!/usr/bin/env python3
"""Convert sqlx `?` placeholders to PostgreSQL `$N` placeholders inside string
literals passed to sqlx::query / query_as / query_scalar and related.

For each match:
  - Find the SQL string literal passed to a `query*` call.
  - Walk the body, tracking single-quote parity to skip `?` inside SQL string
    literals (we don't have any in the codebase, but be safe).
  - Replace each `?` in order with `$1`, `$2`, ...
"""
import re
import sys
from pathlib import Path

# Match a sqlx query call and capture the opening quote+raw-prefix of the SQL
# literal. Body itself is captured by a non-greedy lazy match; we re-locate
# the close quote separately so escapes inside the body don't break us.
PATTERN = re.compile(
    r"""
    (?P<lead>\b(?:sqlx::)?(?:query|query_as|query_as_unchecked|query_scalar|query_scalar_unchecked|query_with)\s*)
    \(\s*
    (?P<raw>r\#+)?(?P<quote>")
    (?P<body>.*?)
    (?P=quote)
""",
    re.VERBOSE | re.DOTALL,
)


def replace_q(sql: str) -> tuple[str, int]:
    """Replace each `?` outside SQL single-quoted literals with `$N`."""
    out = []
    n = 0
    in_single = False
    i = 0
    while i < len(sql):
        c = sql[i]
        if c == "'":
            # SQL doubles single quote to escape: '' inside a string literal.
            in_single = not in_single
            out.append(c)
            i += 1
            continue
        if c == "?" and not in_single:
            n += 1
            out.append(f"${n}")
            i += 1
            continue
        out.append(c)
        i += 1
    return "".join(out), n


def convert_file(path: Path) -> tuple[int, int]:
    src = path.read_text()
    parts = []
    last = 0
    total = 0
    for m in PATTERN.finditer(src):
        body_start, body_end = m.span("body")
        body = src[body_start:body_end]
        converted, n = replace_q(body)
        if n == 0:
            continue
        parts.append(src[last:body_start])
        parts.append(converted)
        last = body_end
        total += n
    if total == 0:
        return 0, 0
    parts.append(src[last:])
    path.write_text("".join(parts))
    return 1, total


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: sqlx_pg_convert.py FILE [FILE ...]", file=sys.stderr)
        return 2
    grand_files, grand_swaps = 0, 0
    for arg in argv[1:]:
        p = Path(arg)
        if not p.is_file():
            continue
        f, s = convert_file(p)
        grand_files += f
        grand_swaps += s
        if s:
            print(f"{p}: swapped {s} placeholders")
    print(f"---\nfiles changed: {grand_files}, placeholders swapped: {grand_swaps}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
