# HyperQuark libAFL fuzzer

This crate fuzzes crashes in `hyperquark` by:

1. Generating `sb3::structured::StructuredProject` values from arbitrary bytes.
2. Converting to `sb3::raw::Sb3Project`.
3. Serializing to JSON.
4. Calling `hyperquark::sb3_to_wasm`.

Crashes found by libAFL are written to `./crashes`.
Recovered crash artifacts are written to `./crash-artifacts` as matching `.json` and `.txt` files.
Artifacts use the same basename as the crash file, for example:

- `crashes/aabbccddeeff0011`
- `crash-artifacts/aabbccddeeff0011.json`
- `crash-artifacts/aabbccddeeff0011.txt`

Initial seeds can be placed in `./corpus` as raw byte files.
The harness will load them automatically at startup.

## Run

```sh
cd fuzz-libafl
cargo run --release
```

## Recover SB3 From Crash File

Crash files in `./crashes` are raw byte inputs. To recover the generated SB3 JSON:

```sh
cd fuzz-libafl
cargo run --release --bin recover_sb3 -- crashes/<crash-file> recovered-project.json
```

If you omit `recovered-project.json`, the JSON is printed to stdout.

## Replay Crash With Backtrace

To replay a crash and print a Rust backtrace in one command:

```sh
cd fuzz-libafl
cargo run --release --bin replay_crash -- crashes/<crash-file> recovered-project.json
```

This command also writes the recovered `project.json` if an output path is provided.
If the panic is reproduced, the process exits with a non-zero status.

## Reduce A Scratch Project

To shrink a `project.json` while keeping another command's output identical:

```sh
cd fuzz-libafl
cargo run --release --bin reduce_project -- -- <command> [args...] --input project.json --output reduced.json
```

The reducer mutates the `sb3::structured::StructuredProject` directly, serializes each candidate back to JSON, and keeps only candidates that are smaller and produce the same command output as the baseline run.

## Notes

- Inputs that fail `StructuredProject` generation or conversion are skipped.
- The fuzzer uses execution time feedback and crash objective (panic/unrecoverable faults).
- If no seed is accepted during startup, the harness inserts fallback byte seeds so fuzzing can begin.
- The runner backfills artifacts for existing crash files at startup and continues watching for new crash files.
- If the process aborts before any crash file is written, there is no input to recover.
