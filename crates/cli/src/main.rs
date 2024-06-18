mod opt;
mod options;

use crate::options::Options;
use anyhow::{bail, Result};
use log::LevelFilter;
use std::env;
use std::process::Stdio;
use std::{fs, io::Write, process::Command};
use structopt::StructOpt;
use tempfile::TempDir;

const CORE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/engine.wasm"));

fn main() -> Result<()> {
    let mut builder = env_logger::Builder::new();
    builder
        .filter(None, LevelFilter::Info)
        .target(env_logger::Target::Stdout)
        .init();

    let opts = Options::from_args();
    if opts.core {
        opt::Optimizer::new(CORE)
            .wizen(true)
            .write_optimized_wasm(opts.output)?;
        return Ok(());
    }

    // Copy in the user's js code from the configured file
    let user_code = fs::read(&opts.input_js)?;

    // Create a tmp dir to hold all the library objects
    // This can go away once we do all the wasm-merge stuff in process
    let tmp_dir = TempDir::new()?;
    let core_path = tmp_dir.path().join("core.wasm");

    // First wizen the core module
    let self_cmd = env::args().next().expect("Expected a command argument");
    {
        let mut command = Command::new(self_cmd)
            .arg("-c")
            .arg(&opts.input_js)
            .arg("-o")
            .arg(&core_path)
            .stdin(Stdio::piped())
            .spawn()?;
        command
            .stdin
            .take()
            .expect("Expected to get writeable stdin")
            .write_all(&user_code)?;
        let status = command.wait()?;
        if !status.success() {
            bail!("Couldn't create wasm from input");
        }
    }

    let output = Command::new("wasm-merge")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if output.is_err() {
        bail!("Failed to detect wasm-merge. Please install binaryen and make sure wasm-merge is on your path: https://github.com/WebAssembly/binaryen");
    }

    // Merge the shim with the core module
    let status = Command::new("wasm-merge")
        .arg(&core_path)
        .arg("core")
        .arg("-o")
        .arg(&opts.output)
        .arg("--enable-reference-types")
        .arg("--enable-bulk-memory")
        .status()?;
    if !status.success() {
        bail!("wasm-merge failed. Couldn't merge shim");
    }

    opt::optimize_wasm_file(opts.output)?;

    Ok(())
}
