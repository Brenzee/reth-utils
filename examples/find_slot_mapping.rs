//! Example: Fast ERC20 slot resolution using known mapping slot
//!
//! This example benchmarks how long it takes to compute the storage slot for a user's
//! ERC20 balance using a known mapping slot index cache.
//!
//! For each of 100 iterations, it generates a random address, computes the expected
//! balance storage slot using `keccak256(abi.encode(user, mapping_slot))`, and measures
//! how long this computation takes using the `reth_utils::get_erc20_balance_slot` helper.
//!
//! At the end, it prints the number of successful queries and the average time per call.
//!
//! Usage:
//! ```bash
//! RETH_DB_PATH=path/to/reth/db cargo run --release --example find_slot_mapping
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use std::{env, time::Instant};

use alloy::primitives::U256;
use reth_utils::erc20::MappingIndexCache;
use reth_utils::{config::Config, erc20::get_erc20_balance_slot};
use revm::primitives::{address, Address};

fn main() -> eyre::Result<()> {
    let db_path = env::var("RETH_DB_PATH")?;
    let config = Config::new(Path::new(&db_path))?;

    // Imitate a cache and store WETH balance mapping slot
    let mut mapping_index_cache: MappingIndexCache = HashMap::new();

    let weth = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
    let weth_mapping_slot = U256::from(3);
    mapping_index_cache.insert(weth, weth_mapping_slot);

    let mut total_duration = Duration::ZERO;
    let mut successful = 0;

    for i in 0..100 {
        let random_user = Address::random();

        let start_t = Instant::now();
        match get_erc20_balance_slot(&config, weth, random_user, Some(&mapping_index_cache)) {
            Ok(_slot) => {
                let elapsed = start_t.elapsed();
                println!("#{i:03} time taken: {:?}", elapsed);

                total_duration += elapsed;
                successful += 1;
            }
            Err(_) => {}
        }
    }

    if successful > 0 {
        let avg = total_duration / successful;
        println!("\nQueries: {successful}");
        println!("⏱ Total time: {:?}", total_duration);
        println!("📊 Average time per successful call: {:?}", avg);
    }

    Ok(())
}
