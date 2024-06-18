use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "apoxy-js", about = "Apoxy JavaScript Edge Function Compiler")]
pub struct Options {
    #[structopt(parse(from_os_str))]
    pub input_js: PathBuf,

    #[structopt(short = "o", parse(from_os_str), default_value = "index.wasm")]
    pub output: PathBuf,

    #[structopt(short = "c")]
    pub core: bool,
}
