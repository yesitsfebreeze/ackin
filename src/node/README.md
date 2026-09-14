# Node

A node: one cartridge's `init.lua`, run by the base in its own process with
the `cartridge` global injected. Everything the entry registers goes to the
base; native modules it loads reach the same global. This is also where a
cartridge's helper programs are spawned and read.