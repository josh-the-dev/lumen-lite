# lumen-lite

Lean Windows tool that keeps Corsair peripheral lighting working without iCUE. It is a small, auditable replacement for iCUE's one job that matters to users of supported devices, not a feature clone.

## Commands

```
cargo fmt                                   # format (CI runs --check)
cargo clippy --all-targets -- -D warnings   # lint; must be clean
cargo test                                  # unit tests, no hardware needed
cargo build --release                       # target/release/*.exe
cargo run -- list                           # enumerate devices, sends nothing
```

Run all of fmt, clippy and test before considering any change done.

## Architecture

- `src/protocol.rs`: pure packet encoding and response parsing. No I/O. Every byte sent to a device is built here and covered by a unit test.
- `src/devices.rs`: table of supported models (USB PID, name, kind, zone count).
- `src/device.rs`: HID I/O via `hidapi` (pure-Rust `windows-native` backend). Discovery, open, request/response with retries.
- `src/watch.rs`: the polling loop. Every 2 s, checks each device's lighting mode and switches it back to hardware mode if needed, with backoff when another program keeps taking control.
- `src/main.rs`: CLI (`list`, `status`, `hardware`, `watch`). Hand-rolled arg parsing; keep it that way unless commands grow substantially.
- `src/bin/agent.rs`: windowless `watch` for running at logon; logs to `%LOCALAPPDATA%\lumen-lite\agent.log`.

## Design principles

- **Enforce hardware mode; don't drive the lighting.** Devices go dark when left in software mode with no host sending colors. Handing control back to the onboard profile fixes that with one request and can't go stale. Host-driven effects are a non-goal unless they're explicitly requested.
- **Polling over OS events.** Polling handles boot, sleep/resume and replugging uniformly with no Win32 message plumbing. Only replace it if there's a measured cost. Rescan with `device::rescan` (vendor-filtered), never `HidApi::refresh_devices`: the unfiltered scan queries every HID device on the system and measured ~100 ms per pass vs <1 ms.
- **Minimal writes.** The only write today is SET lighting mode. Never send firmware, bootloader or profile-storage commands. Any new write needs a byte-level unit test and a note in the README "What it sends" section.
- **Fail soft in the loop.** Per-device errors are logged (deduplicated) and retried; only HID init failure ends `watch`.

## Protocol (Corsair "V2" peripheral)

- Vendor `0x1B1C`, usage page `0xFF42`. Packets: 65-byte writes (report ID `0x00`, then payload); 64-byte reads.
- Payload: `[1]` = `0x08` (wired), `[2]` = command (`0x01` SET, `0x02` GET), `[3..5]` = property (u16 LE), SET value at `[5..9]` (u32 LE).
- Response: `[2]` = status (0 ok; 1 invalid value, 3 failed, 5 unsupported), `[3..7]` = value (u32 LE).
- Property `0x0003` = lighting mode: `0x01` hardware, `0x02` software.

**Unverified on hardware:** GET of the mode property is inferred (public references only SET it). If devices reject it, switch `watch` to re-asserting hardware mode on connect and after detected resume (a wall-clock gap much larger than the poll interval). The supported-device table also needs real-hardware confirmation; update the README status column when confirmed.

## Adding a device

1. Confirm VID/PID and that it uses the V2 protocol on usage page `0xFF42` (`cargo run -- list`).
2. Add a `Model` entry in `src/devices.rs`.
3. Test `status` and `hardware` on real hardware before marking it supported in the README.

## Conventions

- Edition 2024, MSRV in `Cargo.toml` `rust-version`; toolchain pinned to stable via `rust-toolchain.toml`.
- Lints live in `Cargo.toml` `[lints]`: `unsafe_code` forbidden, clippy pedantic on, no `unwrap`/`expect`/`panic` outside tests. Fix lints rather than `#[allow]` them; if an allow is truly needed, scope it narrowly and add a comment explaining why.
- Public functions returning `Result` document `# Errors`; pure getters/constructors are `#[must_use]`.
- Errors: `anyhow` with `.context(...)` at I/O boundaries; typed errors (`ResponseError`) where callers might match on them.
- Keep dependencies minimal; justify any new crate.
- Comments explain *why*, not *what*.

## Legal and licensing

- MIT licensed. Clean-room: implement from protocol facts and our own USB captures. Do not copy or translate code from GPL projects (OpenRGB, ckb-next, OpenLinkHub).
- Never commit Corsair binaries, firmware, SDK files or assets.
- Use Corsair trademarks only descriptively, never in the project name or branding; keep the README disclaimer.

## Git

- Small, focused commits with imperative subject lines.
- Do not add AI co-author or "generated with" trailers to commits or PRs.
