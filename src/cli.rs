use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short = 'f', long = "file", default_value = "compi.toml")]
    pub file: String,

    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    #[arg(long = "rm")]
    pub rm: bool,

    pub task: Option<String>,
}
