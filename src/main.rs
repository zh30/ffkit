use clap::Parser;

use ffkit::cli::Cli;
use ffkit::contract::{Contract, EmitStyle, Status};

fn main() {
    let cli = Cli::parse();
    let style = if cli.json_brief {
        EmitStyle::JsonBrief
    } else if cli.json {
        EmitStyle::Json
    } else {
        EmitStyle::Human
    };

    let contract = match ffkit::run(cli) {
        Ok(c) => c,
        Err(e) => Contract::failed("ffkit", &e),
    };
    style.emit(&contract);
    if matches!(contract.status, Status::Failed) {
        std::process::exit(1);
    }
}
