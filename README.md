# track

Simple time-tracking CLI utility

## Usage

The project name being manipulated is always the positional argument at the end.

**Start tracking a project.** This command starts the timer in the foreground, showing a 1-line progress bar. Press Ctrl-C to exit the tracking.

```bash
track project1
```

When the tracking ends, then the new entry is appended to the time tracking file.

**Set daily goals.** *track* uses the [humantime](https://docs.rs/humantime/latest/humantime/) module to parse human readable time. Pass in the human time in the suitable format.

```bash
track --daily "1h" "project1"
```

The above command will return an output on success.

**Print logs.** These commands always react relative to the local timezone.

```bash
track --today
track --yesterday
track --this-week
track --this-month
track --this-year
track --from time [--to time]
```

### Additional parameters

**Set tracking file**

```
-f/--file [file.jsonl]
```

## Format

*track* uses JSONL to track the time. The tracking file is saved at `~/Documents/track.jsonl`

The first line is always an object containing additional information (goals, categories, ...). It has the following format:

```js
{
  "goals": {
    "project1": 123, // 123 seconds
  },
  "categories": ["project1"],
}
```

Subsequent lines are arrays representing time entries. Time is always represented using Unix timestamps in UTC.

```js
["category", start_time, end_time]
```

*track* never ends the JSONL file with a newline. when appending a new entry, a newline is appended to the file, and then the JSON encoded entry is appended. If the file is empty, then a newline character is not appended at the initial position.

## License

MIT
