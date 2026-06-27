# Conventions

- *track* is a time-tracking CLI tool written in Rust
- Prioritize simple code with minimal dependencies.
- Add comments ONLY if it's not immediately obvious from a cursory glance of the code.
- When commiting, try to summarize what the commit does within only the commit title. If your commit is long, use bullet points to describe the contents in the body.

## Time

- Unless you're working directly with the raw database, use the `OffsetDateTime`, etc. structs of the `time` crate. Do NOT use UNIX timestamps unless the explicitly requested.
- Do not use the native `SystemTime`, `Duration`. Use the `time` crate instead.
- To get the current local time, use `crate::utils::time::now_local()` returning `OffsetDateTime`.
- For functions facilitating CLI interactions, ALWAYS use local time.
- Use `crate::utils::time::DATETIME_FMT` (a `LazyLock` format description) instead of calling `time::format_description::parse` repeatedly. It formats as `[year]-[month]-[day] [hour]:[minute]:[second]`.
- When merging databases with `--merge-from`, the merged entries count and date range (from the common ancestor entry to the latest source entry) are printed to stdout.
- To format `time::Duration`, use the `unsigned_abs` function to convert to the standard `Duration`, then call `humantime::format_duration`:

```rust
humantime::format_duration(elapsed.unsigned_abs())
```

