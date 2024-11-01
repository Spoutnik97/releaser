use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(value_name = "ENVIRONMENT", default_value = "production")]
    pub environment: String,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub tag: bool,
}
