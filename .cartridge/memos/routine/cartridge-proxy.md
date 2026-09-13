---
kind: routine
description: Start an authenticated local proxy with a private persisted development key and a disposable per-port profile.
---

# Start the local proxy

Use `just proxy <port>`. The explicit environment key takes precedence; otherwise
keep a generated key in ignored `.cartridge/dev/` with private permissions.
Never print the key or commit the generated profile. A busy port refuses before
starting another daemon.

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
    const profile=path.join(dev,"proxy-"+port),base=path.join(runtime,".cartridge/proxy");
    fs.mkdirSync(profile,{recursive:true});fs.copyFileSync(path.join(base,"init.lua"),path.join(profile,"init.lua"));
    fs.writeFileSync(path.join(profile,"config.lua"),"local config = dofile("+JSON.stringify(path.join(base,"config.lua"))+")\nconfig.proxy.listen = \"127.0.0.1:"+port+"\"\nconfig.router.listen = {\"127.0.0.1:0\"}\nreturn config\n");
    const binary=path.join(process.env.CARGO_TARGET_DIR||path.join(runtime,"target"),"debug/cartridge");
    const child=Bun.spawn([binary,"--profile",profile,"daemon"],{cwd:runtime,env:process.env,stdin:"inherit",stdout:"inherit",stderr:"inherit"});
    for(const signal of ["SIGINT","SIGTERM"])process.on(signal,()=>child.kill(signal));
    process.exit(await child.exited);
```
