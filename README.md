# lumen-lite

A tiny Windows tool that keeps Corsair keyboard and mouse lighting on, without iCUE.

## Why

Corsair devices run in one of two modes:

- **Hardware mode**: the device plays the lighting profile saved in its own memory.
- **Software mode**: the PC drives the lighting.

iCUE puts devices in software mode. When it crashes, starts late after boot, or loses the device after sleep, nothing is sending colors and the lights go dark until you tick "device memory mode" again.

lumen-lite checks each device's mode every couple of seconds and hands control back to the onboard profile if it's been left in software mode. It uses almost no CPU or memory.

## Supported devices

| Device | USB ID | Status |
|---|---|---|
| K55 RGB PRO | `1B1C:1BA4` | Needs testing on real hardware |
| Scimitar RGB Elite | `1B1C:1BE3` | Needs testing on real hardware |

## Usage

```
lumen-lite list        # show connected Corsair devices (sends nothing to them)
lumen-lite status      # show which lighting mode each device is in
lumen-lite hardware    # switch every device back to its onboard profile
lumen-lite watch       # keep devices in hardware mode (Ctrl+C to stop)
```

`lumen-lite-agent.exe` does the same as `watch` with no console window, logging to `%LOCALAPPDATA%\lumen-lite\agent.log`.

### Start at logon

1. Save your lighting to the device's onboard memory once in iCUE.
2. Stop iCUE starting with Windows (or uninstall it). If both run, they'll fight over the device; lumen-lite notices and backs off for 10 minutes at a time.
3. Register the agent:

   ```
   reg add HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v lumen-lite /d "C:\path\to\lumen-lite-agent.exe"
   ```

## What it sends

Only two kinds of request, both on the device's vendor control channel:

- **GET lighting mode**: reads which mode the device is in.
- **SET lighting mode to hardware**: the same switch iCUE's "device memory mode" makes.

It never writes firmware, profiles, or anything else to the device's memory. See [`src/protocol.rs`](src/protocol.rs) for the exact bytes.

## Building

```
cargo build --release
```

Requires Rust and the MSVC build tools.

## Disclaimer

Not affiliated with or endorsed by Corsair. "Corsair", "iCUE", "K55" and "Scimitar" are trademarks of their respective owners and are used here only to describe compatible hardware.

## License

MIT
