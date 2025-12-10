use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "zk-mpc-node", about = "usage of zk-mpc-node commands.")]
pub enum Command {
    /// generate keypair for the node
    #[structopt(name = "keygen")]
    KeyGen {
        /// node ID
        #[structopt(long)]
        id: u32,
    },
    /// start the node
    #[structopt(name = "start")]
    Start {
        /// node ID
        #[structopt(long)]
        id: u32,
    },
}
