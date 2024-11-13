use reqwest::blocking::get;
use reqwest::StatusCode;
use rust_decimal::Decimal;
use serde_json::Value;
use std::error::Error;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::thread::sleep;
use std::time::Duration;

// use bitcoin::base58;
use bitcoin::bip32::{DerivationPath, Xpub};
use bitcoin::key::CompressedPublicKey;
use bitcoin::secp256k1::{All, Secp256k1};
use bitcoin::{Address, Network};
use std::str::FromStr;
use xyzpub::{convert_version, Version};

const GAP_LIMIT: u8 = 5;
// Testnet address prefixes
const PREFIX_VPUB: &str = "vpub";
const PREFIX_TPUB: &str = "tpub";
const PREFIX_UPUB: &str = "upub";

// Mainnet address prefixes
const PREFIX_XPUB: &str = "xpub";
const PREFIX_YPUB: &str = "ypub";
const PREFIX_ZPUB: &str = "zpub";

const SATS_IN_BTC: i64 = 100_000_000;

pub struct Config {
    pub xpub_key: String,
    pub gap_limit: u8,
}

impl Config {
    pub fn build(args: &Vec<String>) -> Result<Config, &str> {
        if args.len() < 2 {
            return Err("Not enough arguments");
        }

        let xpub_key = args[1].clone();
        let gap_limit: u8;

        if args.len() == 3 {
            gap_limit = args[2]
                .parse::<u8>()
                .map_err(|_| "Failed to parse gap_limit argument")?;
        } else {
            gap_limit = GAP_LIMIT;
        }

        Ok(Config {
            xpub_key,
            gap_limit,
        })
    }
}

struct BtcAddress {
    address: String,
    path_suffix: String, // "0/0", "1/0" etc.
    balance: u64,        // sats
    balance_query_successful: bool,
}

impl BtcAddress {
    fn new(
        address: String,
        path_suffix: String,
        balance: u64,
        query_successful: bool,
    ) -> BtcAddress {
        BtcAddress {
            address: address,
            path_suffix: path_suffix,
            balance: balance,
            balance_query_successful: query_successful,
        }
    }
}

fn generate_address(xpub_key: Xpub, path: &str) -> Result<String, Box<dyn Error>> {
    let secp: Secp256k1<All> = Secp256k1::new();
    let path = DerivationPath::from_str(path)?;
    let child_pub_key = xpub_key.derive_pub(&secp, &path)?;
    let compressed_pub_key =
        CompressedPublicKey::from(CompressedPublicKey(child_pub_key.public_key));
    let address = Address::p2wpkh(&compressed_pub_key, Network::Testnet);
    Ok(address.to_string())
}

fn generate_addresses_and_get_balances(
    config: &Config,
    addresses: &mut Vec<BtcAddress>,
    base_url: &str,
) -> Result<(), Box<dyn Error>> {
    // NOTE: seems odd to have to convert but doesn't seem to be working without
    // conversion. Needs more digging.
    let tpub_string = convert_version(config.xpub_key.clone(), &Version::Tpub).unwrap();
    println!("converted address {tpub_string}");
    let xpub_key = Xpub::from_str(&tpub_string)?;
    // let secp: Secp256k1<All> = Secp256k1::new();

    // Checking for change= 0 (receiving) and 1 (change) addresses
    for i in 0..2 {
        let mut num_of_contiguous_unused_addr = 0;
        let mut j = 0;

        loop {
            let addr_str = generate_address(xpub_key, &format!("{i}/{j}"))?;

            let mut balance = 0;
            let mut success = true;

            match get_address_balance(&addr_str, base_url) {
                Ok(val) => balance = val,
                Err(e)
                    if e.downcast_ref::<reqwest::Error>().map_or(false, |err| {
                        err.status() == Some(StatusCode::TOO_MANY_REQUESTS)
                    }) =>
                {
                    // breaking here to avoid looping forever
                    eprintln!("status code 429, rate limit hit, breaking out");
                    break;
                }
                Err(_) => success = false,
            }

            let btc_address = BtcAddress::new(addr_str, format!("{i}/{j}"), balance, success);
            addresses.push(btc_address);

            if success {
                if balance == 0 {
                    num_of_contiguous_unused_addr += 1;
                } else {
                    num_of_contiguous_unused_addr = 0;
                }
            }

            if num_of_contiguous_unused_addr == config.gap_limit {
                break;
            }
            j += 1;
        }
        println!("");
    }
    Ok(())
}

fn get_base_url(xpub_key: &str) -> Result<&str, Box<dyn Error>> {
    if xpub_key.starts_with(PREFIX_TPUB)
        || xpub_key.starts_with(PREFIX_UPUB)
        || xpub_key.starts_with(PREFIX_VPUB)
    {
        return Ok("https://api.blockcypher.com/v1/btc/test3/addrs/");
    } else if xpub_key.starts_with(PREFIX_XPUB)
        || xpub_key.starts_with(PREFIX_YPUB)
        || xpub_key.starts_with(PREFIX_ZPUB)
    {
        return Ok("https://api.blockcypher.com/v1/btc/main/addrs/");
    }

    Err(Box::new(IoError::new(
        ErrorKind::InvalidInput,
        "xpub_key does not have a known prefix",
    )))
}

// This function can be fully async which should increase performance
// significantly since this is primarily an I/O operation
fn get_address_balance(address: &str, base_url: &str) -> Result<u64, Box<dyn Error>> {
    // To not hit rate limits of free APIs
    sleep(Duration::from_millis(500));

    let url = format!("{}/balance", base_url.to_owned() + address);
    let response = get(url)?;
    let balance: u64;

    if !response.status().is_success() {
        return Err(response.error_for_status().unwrap_err().into());
    }

    let resp_json: Value = response.json()?;
    match resp_json["final_balance"].as_u64() {
        Some(val) => balance = val,
        None => return Err("Failed to parse response json".into()),
    }

    Ok(balance)
}

fn print_balances(addresses: &Vec<BtcAddress>) {
    let mut sum = 0;

    if addresses.len() == 0 {
        return;
    }
    println!("");
    println!("{:<12} {:<50} {:<20}", "Path_Suffix", "Address", "Balance");
    for addr in addresses {
        if addr.balance_query_successful {
            println!(
                "{:<12} {:<50} {:<20}",
                addr.path_suffix, addr.address, addr.balance
            );
        } else {
            // clearly indicate when balance could not be fetched.
            println!(
                "{:<12} {:<50} {:<20}",
                addr.path_suffix, addr.address, "Unavailable"
            );
        }
        sum += addr.balance;
    }

    let btc = Decimal::from(sum) / Decimal::new(SATS_IN_BTC, 0);
    println!("Total Combined Balance: {} sats, {} btc", sum, btc);
}

pub fn run(c: Config) -> Result<(), Box<dyn Error>> {
    println!("address {}, gap_limit {}", c.xpub_key, c.gap_limit);
    let mut addresses: Vec<BtcAddress> = vec![];
    let base_url = get_base_url(&c.xpub_key)?;

    generate_addresses_and_get_balances(&c, &mut addresses, base_url)?;
    print_balances(&addresses);

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{get_base_url, PREFIX_TPUB, PREFIX_VPUB};

    #[test]
    fn check_url() {
        let keys = vec![
            "tpubDCpP2bUR4GbTZkfizWRozZVuZ2aohedBEzpHzvckRFXvKDWko6kA4T3PdUsFgXL9qtJ8326v52uwxG6HCMkA9fPym6QkiUgjqKyDx1eHAgy",
            "vpub5YnDu2Ju3dZ3bN6dsbsUNTyXsyCFq297s9BZ5amqKL2GTjDbDZZwft4HM2sJAD55EhXbvVPvccNoVWNYN74tfkaUxpGbs8PXhvFXQmgCrAA",
        ];
        for key in keys {
            let expected = "https://api.blockcypher.com/v1/btc/test3/addrs/";

            match get_base_url(key) {
                Ok(base_url) => assert_eq!(base_url, expected),
                Err(_) => panic!("Failed for key {}", key),
            }
        }
    }
}
