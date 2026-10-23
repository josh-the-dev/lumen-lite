use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use hidapi::HidApi;
use lumen_lite::device::{self, Device};
use lumen_lite::protocol::{Mode, VENDOR_ID};
use lumen_lite::{devices, timestamped, watch};

const USAGE: &str = "\
lumen-lite: keeps Corsair lighting on, without iCUE

USAGE:
    lumen-lite <COMMAND>

COMMANDS:
    list        Show connected Corsair HID interfaces (sends nothing to them)
    status      Show which lighting mode each supported device is in
    hardware    Hand lighting back to each device's onboard profile
    watch       Keep devices in hardware mode, checking every 2 seconds
    help        Show this message
";

fn main() -> ExitCode {
    let result = match std::env::args().nth(1).as_deref() {
        Some("list") => list(),
        Some("status") => status(),
        Some("hardware") => hardware(),
        Some("watch") => watch::run(|msg| println!("{}", timestamped(msg))),
        None | Some("help" | "-h" | "--help") => {
            print!("{USAGE}");
            Ok(())
        }
        Some(other) => {
            eprint!("unknown command `{other}`\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn list() -> Result<()> {
    let api = HidApi::new().context("couldn't initialise HID")?;
    let mut any = false;
    for info in api
        .device_list()
        .filter(|info| info.vendor_id() == VENDOR_ID)
    {
        any = true;
        let support = devices::by_pid(info.product_id()).map_or("unsupported", |model| model.name);
        println!(
            "{:04x}  interface {:>2}  usage {:04x}:{:04x}  {:<32}  [{support}]",
            info.product_id(),
            info.interface_number(),
            info.usage_page(),
            info.usage(),
            info.product_string().unwrap_or("?"),
        );
    }
    if !any {
        println!("No Corsair devices found.");
    }
    Ok(())
}

fn status() -> Result<()> {
    for_each_device(|device| device.mode().map(|mode| mode.to_string()))
}

fn hardware() -> Result<()> {
    for_each_device(|device| {
        device.set_mode(Mode::Hardware)?;
        Ok("switched to onboard profile".to_owned())
    })
}

/// Runs `action` on every supported device, reporting each result. Keeps
/// going past failures but returns an error if any device failed, so
/// scripts can rely on the exit code.
fn for_each_device(mut action: impl FnMut(&Device) -> Result<String>) -> Result<()> {
    let api = HidApi::new().context("couldn't initialise HID")?;
    let found = device::discover(&api);
    if found.is_empty() {
        println!("No supported devices found. Run `lumen-lite list` to see what's connected.");
    }
    let mut failed = 0;
    for f in &found {
        match Device::open(&api, f).and_then(|device| action(&device)) {
            Ok(msg) => println!("{}: {msg}", f.model.name),
            Err(err) => {
                eprintln!("{}: error: {err:#}", f.model.name);
                failed += 1;
            }
        }
    }
    if failed > 0 {
        bail!("{failed} of {} devices failed", found.len());
    }
    Ok(())
}
