use std::env::args;
use std::process;
use wallet_balance::{run, Config};

fn print_usage() {
    const MSG: &str = r#"
USAGE:
    wallet_balance <xpub_key> [gap_limit]

    xpub_key    Extended public key
    gap_limit   Number of zero balance addresses after which the tool stops scanning 
                defaults to 5
    "#;
    println!("{}", MSG);
}

fn main() {
    let args: Vec<String> = args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        println!("{}", err);
        print_usage();
        process::exit(1);
    });

    if let Err(e) = run(config) {
        println!("Failed to run: {}", e);
        process::exit(1);
    }
}
