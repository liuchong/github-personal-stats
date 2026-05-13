use github_stats_core::{parse_output_kind, workspace_info};
use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("info") | None => println!("{}", workspace_info().to_json()),
        Some("generate") => {
            let card = read_card_argument(args.collect());
            let output = parse_output_kind(&card)?;
            println!("{}", output.as_str());
        }
        Some(command) => return Err(format!("unsupported command: {command}").into()),
    }
    Ok(())
}

fn read_card_argument(args: Vec<String>) -> String {
    args.windows(2)
        .find_map(|window| (window[0] == "--card").then(|| window[1].clone()))
        .unwrap_or_else(|| "dashboard".to_owned())
}
