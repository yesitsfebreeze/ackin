# Sandbox

The wall a cartridge's `grant` builds around its child process. On macOS the
policy is `sandbox-exec`; on Linux it is Landlock plus a seccomp socket
filter, installed by the `__confine` trampoline in `linux.rs`. A child that
cannot be confined on the running platform is not started at all.