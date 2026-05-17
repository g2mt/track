#!/usr/bin/env python3
"""Migrate the info line of a track data file from the old schema to the new one.

Usage: python3 utils/migrate.py <data_file>

Old schema: {"goals": {"cat": secs}, "categories": ["cat", ...]}
New schema: {"categories": {"cat": {"goal": secs_or_null}, ...}}
"""
import json
import sys


def migrate(filepath: str) -> None:
    with open(filepath, "r") as f:
        content = f.read()

    # Find end of first line
    nl = content.find("\n")
    if nl == -1:
        print("ERROR: file has no newline", file=sys.stderr)
        sys.exit(1)

    first_line = content[:nl]
    rest = content[nl:]

    old = json.loads(first_line)

    # Build new categories map
    goals = old.get("goals", {})
    categories = old.get("categories", [])
    new_categories = {}
    for cat in categories:
        secs = goals.get(cat)
        new_categories[cat] = {"goal": secs} if secs is not None else {"goal": None}

    # Preserve any old categories not in the list (shouldn't happen, but be safe)
    for cat, secs in goals.items():
        if cat not in new_categories:
            new_categories[cat] = {"goal": secs}

    new_info = {"categories": new_categories}
    new_first_line = json.dumps(new_info, separators=(",", ":"))

    # Preserve original line length padding
    old_len = len(first_line)
    new_len = len(new_first_line)
    if new_len <= old_len:
        new_first_line = new_first_line.ljust(old_len)
    # else: new line longer, shift will happen naturally on next write_info

    with open(filepath, "w") as f:
        f.write(new_first_line)
        f.write(rest)

    print(f"Migrated {filepath}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <data_file>", file=sys.stderr)
        sys.exit(1)
    migrate(sys.argv[1])
