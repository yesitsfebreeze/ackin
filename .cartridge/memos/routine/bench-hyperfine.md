---
kind: routine
description: Benchmark command-line programs with hyperfine — timed runs with warmup, statistical mean/stddev,
  and A/B comparison of two commands. Use to measure how long a command takes, compare two implementations,
  or check a speedup with real numbers.
uses:
- usage: '[[run-usage]]'
  when:
  - how long does this command take
  - benchmark a command
  - compare the speed of two commands
  - measure performance
  - is this faster
  - time a program with warmup
  tags:
  - bench
  - hyperfine
  - benchmark
  - timing
  - performance
  - compare
---

## Inputs

Requires on PATH: `hyperfine`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Statistical timing for commands — runs each many times, reports mean ± stddev,
and ranks an A/B comparison with a speedup factor. Far better than `time`, which
gives one noisy sample. `run` benchmarks one command; `compare` ranks several;
`warmup` primes caches first so you measure steady state, not cold start.

| Recipe | Does |
|--------|------|
| `run` | benchmark one command (mean, stddev, min/max) |
| `compare` | run several commands and rank them with a speedup |
| `warmup` | N warmup runs before timing (prime disk/page cache) |
| `export` | write results to a file (json/markdown/csv) |

Each command argument is a full shell string — quote it. Use `-p`/`--prepare`
for per-run setup (e.g. clearing a build) that should not count toward the time.
`compare` is the daily driver: it prints "command A ran 1.34× faster than B"
with confidence, which is the honest way to claim a speedup.

```just
# benchmark one command (default)
run +command:
  hyperfine {{quote(command)}}

# rank two or more commands with a speedup factor (each a quoted string)
compare +commands:
  hyperfine {{commands}}

# N warmup runs before timing, to measure warm steady-state
warmup n command:
  hyperfine --warmup {{quote(n)}} {{quote(command)}}

# write results to a file; format is json | csv | markdown
export format out command:
  hyperfine --export-{{format}} {{quote(out)}} {{quote(command)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `run` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
