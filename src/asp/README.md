# ASP

ASP (Agent Server Protocol) is the agent's one interface to the world, and it is the fabric with a better protocol: the same single graph of everything the composition can reach, now with typed entity ids, types each cartridge declares in its `cartridge.json`, a contributor and revision on every fact, staleness, actions and one ranked search. It lives in the base because every agent depends on it and because only the base knows which cartridges are loaded; providers stay isolated cartridges that answer on their own `asp.<name>` event. `docs/asp.txt` is the protocol.
