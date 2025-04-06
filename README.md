# Reth Utils

## ERC20 Find Slot

Examples ran on release build.

#### `find_slot` example

Token: WETH (only 1 storage slot touched)

```
Queries: 100
⏱ Total time: 11.271966ms
⏱ First call time: 238.797µs
📊 Average time per successful call: 112.719µs
```

Token: USDC (multiple storage slots touched, determines the best one)

```
Queries: 100
⏱ Total time: 18.260411ms
⏱ First call time: 470.98µs
📊 Average time per successful call: 182.604µs
```

#### `find_slot_mapping` example

Token: WETH

```
Queries: 100
⏱ Total time: 38.825µs
📊 Average time per successful call: 388ns
```
