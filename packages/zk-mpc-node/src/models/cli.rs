use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
pub struct Opt {
    pub id: usize,
    #[structopt(parse(from_os_str))]
    pub input: PathBuf,
}
