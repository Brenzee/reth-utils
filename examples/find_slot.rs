//! Example: Detect balance slot for a randomly generated address
//!
//! This example runs 100 iterations for 100 random addresses,
//! then attempts to detect the ERC20 balance storage slot by simulating a `balanceOf()` call
//! using Reth and REVM on an ERC20 token.
//!
//! It measures and prints how long each call takes, and then reports the average time
//! across all successful detections.
//!
//! Usage:
//! ```bash
//! RETH_DB_PATH=path/to/reth/db cargo run --release --example find_slot
//! ```

use std::path::Path;
use std::time::Instant;
use std::{env, time::Duration};

use reth_utils::{config::Config, erc20::get_erc20_balance_slot};
use revm::primitives::{address, Address};

fn main() -> eyre::Result<()> {
    let db_path = env::var("RETH_DB_PATH")?;
    let config = Config::new(Path::new(&db_path))?;

    // USDC
    let erc20 = address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
    let mut total_duration = Duration::ZERO;
    let mut successful = 0;

    for i in 0..100 {
        let random_user = Address::random();

        let start_t = Instant::now();
        match get_erc20_balance_slot(&config, erc20, random_user, None) {
            Ok(_slot) => {
                let elapsed = start_t.elapsed();
                println!("#{i:03} time taken: {:?}", elapsed);
                total_duration += elapsed;
                successful += 1;
            }
            Err(err) => {
                println!("#{i:03} error: {:?}", err);
            }
        }
    }

    if successful > 0 {
        let avg = total_duration / successful;
        println!("\n✅ Successful queries: {successful}");
        println!("⏱ Total time: {:?}", total_duration);
        println!("📊 Average time per successful call: {:?}", avg);
    }

    Ok(())
}
