# track

Simple time-tracking CLI utility

## Usage

The category name being manipulated is always the positional argument at the end.

**Start tracking a category.** This command starts the timer in the foreground, showing a 1-line progress bar. Press Ctrl-C to exit the tracking.

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

The first line is always an object containing additional information (goals, categories, ...). This object is called the info object, and it follows this template:

```js
{
  "goals": {
    "project1": 123, // 123 seconds
  },
  "categories": ["project1"],
}
```

The first line is padded by white spaces, such that the length of the first line is a multiple of 128. For instance, if the object is serialized to 200 characters, then the first line will end with $$256 - 200 - 1 = 55$$ white space characters, and the new line `\n` character.

Subsequent lines are arrays representing time entries, and are not padded by white spaces. Time is always represented using Unix timestamps in UTC.

```js
["category", start_time, end_time]
```

*track* never ends the JSONL file with a new line. When appending a new entry, then the JSON encoded entry is appended, and the new line character is added.

## License

MIT
