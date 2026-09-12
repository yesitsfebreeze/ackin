---
kind: routine
description: Control process scheduling priority — launch a command at low CPU priority, change the priority
  of a running process, and lower its disk-I/O priority. Use to keep a heavy background job from starving
  the system, or to deprioritize a runaway process you found.
uses:
- usage: '[[run-usage]]'
  when:
  - run a command at low priority
  - deprioritize a process
  - stop a job hogging the cpu
  - change process priority
  - lower disk io priority
  - make a background task yield
  tags:
  - proc
  - nice
  - renice
  - ionice
  - priority
  - scheduling
---

## Inputs

Requires on PATH: `coreutils`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects process; danger low.

## Do

Adjust how much CPU (and disk) a process gets relative to others. `run` launches
a command at low priority; `renice` adjusts one already running (PID from
[[proc-ps]] / [[proc-top]]); `io` lowers its disk-I/O class; `high` raises priority
(needs root). The niceness scale runs **−20 (greediest) to +19 (most yielding)**;
positive values are the polite background range.

| Recipe | Tool | Does |
|--------|------|------|
| `run` | `nice -n N` | start a command at niceness N (default +10) |
| `renice` | `renice` | change a running PID's niceness |
| `io` | `ionice -c3` | set a PID to idle disk-I/O priority |
| `high` | `nice -n -5` | start at higher priority (needs root for negatives) |

Higher niceness = *lower* priority (it is "nice to others"). +10 to +19 is the
range for a backup, build, or batch job that should yield to interactive work.
Only root can set *negative* niceness or raise an existing process's priority —
an unprivileged `renice` can only make a process nicer, never greedier.
`ionice -c3` (idle) is the disk equivalent: the job touches the disk only when
nothing else wants it — ideal for a `tar`/`rsync` that must not stall the box.

```just
# start a command at low CPU priority (default niceness +10)
run n="10" +command:
  nice -n {{quote(n)}} {{command}}

# change the niceness of a running PID (positive = lower priority)
renice n pid:
  renice -n {{quote(n)}} -p {{quote(pid)}}

# set a running PID to idle disk-I/O priority
io pid:
  ionice -c3 -p {{quote(pid)}}

# start a command at higher priority (negative niceness; needs root)
high n="-5" +command:
  sudo nice -n {{quote(n)}} {{command}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `run` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
