---
kind: routine
description: Start an authenticated local proxy on a chosen port with a private persisted development key.
---

# Start the local proxy

Use `just proxy <port>`. The explicit environment key takes precedence; otherwise
keep a generated key in ignored `.cartridge/dev/` with private permissions. Never
print the key. A busy port refuses before starting another daemon. There is one
user profile and this recipe does not write a second one: the key and the address
travel in the environment, and `.cartridge/config.lua` binds the proxy listener
only when one of them names it.

```just
set positional-arguments
serve port="4242":
    #!/usr/bin/env bun
    import fs from "node:fs";
    import path from "node:path";
    import net from "node:net";
    import { randomBytes } from "node:crypto";
    const runtime=process.env.MEMO_OWNER_ROOT, port=Number(process.argv[2]);
    if(!Number.isInteger(port)||port<1||port>65535)throw Error("Port must be 1..65535");
    await new Promise((resolve,reject)=>{const probe=net.createServer();probe.once("error",reject);probe.listen(port,"127.0.0.1",()=>probe.close(resolve));});
    const dev=path.join(runtime,".cartridge/dev");fs.mkdirSync(dev,{recursive:true,mode:0o700});
    if(!process.env.CARTRIDGE_PROXY_KEY){
      const key=path.join(dev,"proxy.key");
      try{fs.writeFileSync(key,randomBytes(32).toString("hex")+"\n",{flag:"wx",mode:0o600});}catch(e){if(e.code!=="EEXIST")throw e;}
      process.env.CARTRIDGE_PROXY_KEY=fs.readFileSync(key,"utf8").trim();
      if(!process.env.CARTRIDGE_PROXY_KEY)throw Error("Empty proxy key file");
      console.error("Proxy key file: "+key);
    }
    process.env.CARTRIDGE_PROXY_LISTEN="127.0.0.1:"+port;
    const binary=path.join(process.env.CARGO_TARGET_DIR||path.join(runtime,"target"),"debug/cartridge");
    const child=Bun.spawn([binary,"daemon"],{cwd:runtime,env:process.env,stdin:"inherit",stdout:"inherit",stderr:"inherit"});
    for(const signal of ["SIGINT","SIGTERM"])process.on(signal,()=>child.kill(signal));
    process.exit(await child.exited);
```
