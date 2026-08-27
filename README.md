# dormire

> `sleep`, but you can see it happening. And written in Rust.

**dormire** is a drop-in `sleep` clone with a live progress bar, plus two superpowers the original never had: sleep **until a wall-clock time** and sleep **until a process exits**.

```
$ dormire 5
█████████████████████░░░░░░░░░░░░░░░░░░░  53% 00:02/00:05 (5s)
```
## Installation

```sh
cargo install --path .
# or
cargo build --release && cp target/release/dormire ~/.local/bin/
```

## Usage

### Classic sleep (with a progress bar)

Fully compatible with GNU sleep duration syntax.

```sh
dormire 5            # 5 seconds
dormire 1.5          # decimals work
dormire 2m 30s       # arguments are summed: 2m30s
dormire 1h 15m       # supported suffixes: s, m, h, d (default: s)
```

### Sleep until a time

```sh
dormire --until 14:30              # today, or tomorrow if 14:30 already passed
dormire --until 06:00:00           # seconds optional
dormire --until "2026-09-01 09:00" # an absolute date
```

All times are local.

### Sleep until a process exits

```sh
./long-build & dormire --pid $! && echo "build finished"
```

dormire watches the PID and returns the instant it's gone, no more `while kill -0 $pid; do sleep 1; done` loops.
