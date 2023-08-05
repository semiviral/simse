use std::path::PathBuf;

#[derive(clap::Parser)]
#[command(rename_all = "snake_case")]
pub struct Arguments {
    #[arg(long = "config.file", env = "SIMSE_CONFIG_PATH", required = true)]
    pub config_path: PathBuf,
}
