# Reth Utils

## ERC20 Find Slot

Examples ran on release build.

#### `find_slot` example

Token: WETH (only 1 storage slot touched)

```
✅ Successful queries: 100
⏱ Total time: 10.738619ms
📊 Average time per successful call: 107.386µs
```

Token: USDC (multiple storage slots touched, determines the best one)

```
✅ Successful queries: 100
⏱ Total time: 18.281425ms
📊 Average time per successful call: 182.814µs
```

#### `find_slot_mapping` example

Token: WETH

```
✅ Successful queries: 100
⏱ Total time: 38.825µs
📊 Average time per successful call: 388ns
```
