// tools/gen_completions.rs
use std::env;
use std::fs;

use clap_complete::{
    generate_to,
    shells::{Bash, Fish, Zsh},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::args().nth(1).unwrap_or_else(|| "man".into());
    fs::create_dir_all(&out_dir)?;
    let mut cmd = oc_rsync_cli::cli_command();
    generate_to(Bash, &mut cmd, "oc-rsync", &out_dir)?;
    generate_to(Zsh, &mut cmd, "oc-rsync", &out_dir)?;
    generate_to(Fish, &mut cmd, "oc-rsync", &out_dir)?;
    Ok(())
}
