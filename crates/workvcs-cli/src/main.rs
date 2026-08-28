use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "workvcs")]
#[command(about = "WorkVCS command shell placeholder for Phase 1")]
struct Cli {
    #[arg(long, hide = true)]
    version_probe: bool,
}

fn main() {
    let _cli = Cli::parse();
}
