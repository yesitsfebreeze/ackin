# host

The host runs cartridges. A cartridge is a folder with a `cartridge.json` and a
Lua entry, installed by placing it under the cartridge root and enabled by
naming it in `.cartridge/init.lua`. Every cartridge serves on its own socket;
the host starts them in dependency order and hands each the sockets and tokens
it may use.

## Use

- `cartridge help [<id>[/<file>[#<section>]]]`: the documentation of the host and every cartridge.
- `cartridge list`: the profile and what each need binds to.
- `cartridge settings [<id>|<id>.<key>]`: every setting and the file that settled it.
- `cartridge run <key> '<json>'`: start the profile, call one key, stop.
- `cartridge daemon`, then `cartridge status`, `call`, `send`, `follow`, `reload`, `stop`.
- `cartridge verify [<id>]`: run declared contracts.

## Read next

- [Architecture](../docs/architecture.txt)
- [Transport](../docs/transport.txt)
- [Creating cartridges](../docs/creating-cartridges.txt)
- [Writing good cartridges](../docs/writing-good-cartridges.txt)
- [Settings](../docs/settings.txt)
- [Development](../docs/development.txt)
- [Guide for models](../llms.txt)
