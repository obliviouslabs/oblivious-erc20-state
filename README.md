# Oblivious ERC20 State

This project demonstrates an application that uses [obliviouslabs/ordb](https://github.com/obliviouslabs/ordb) and [obliviouslabs/verified-contract-state](https://github.com/obliviouslabs/verified_contract_state) to access ERC20 token state obliviously within a Trusted Domain eXecution (TDX) environment.

---

## Components

1. **This folder**: 
   - HTTP API server
   - TDX attestation generation
   - DApp Docker image

2. **[obliviouslabs/ordb](https://github.com/obliviouslabs/ordb)**: Oblivious database.

3. **[obliviouslabs/verified-contract-state](https://github.com/obliviouslabs/verified_contract_state)**: Code for fetching Ethereum state from an untrusted Geth-compatible server.

---
### Prerequisites

Ensure the following are installed:

1. **[Docker](https://www.docker.com/)**: For building and running the project.
2. **[Foundryup](https://book.getfoundry.sh/getting-started/installation)** and `cast`: For querying wallet balances.
3. **[Optional] tappd service or [Leechael/tappd-simulator](https://github.com/Leechael/tappd-simulator)**: To generate attestation reports for queries (otherwise every attestation will be empty and a "Connection Refused" error will appear in Docker logs).

---

### Build & Run

1. Create a `.env` file in the root directory (next to the `docker-compose.yml` file).

2. Add a Geth-compatible API server URL to `.env`, e.g.:
   ```env
   GETH_URL=https://eth-mainnet.g.alchemy.com/v2/your_api_key
3. Build and deploy the project using Docker:
   ```bash
   docker build -t oblivious_erc20_state .
   docker-compose up
The server will start running, listening on port 3000. 
The first time it starts, it should take a bit to fetch and verify all the memory addresses using geth api. Afterwards, it will cache them in the checkpoints folder and be much faster (it still uses verified_contract_state to check for address integrity).

---

## Making Requests

### Available APIs

The server exposes two API types:
- **Unquoted API**: Does not generate TDX quotes.
- **Quoted API**: Available under `/quoted` (see `src/main.rs`).

#### Example Requests

1. **Status Check**:
   ```bash
   curl http://localhost:3000/quoted/status -v
2.  **Address Storage Query** (e.g., zero address):
    ```
    curl http://localhost:3000/storage_at -X POST -H 'Content-Type: application/json' --data '{"addr": "0000000000000000000000000000000000000000000000000000000000000000"}'
3. **Quoted Address Storage Query:**
    ```
    curl http://localhost:3000/quoted/storage_at -X POST -H 'Content-Type: application/json' --data '{"addr": "0000000000000000000000000000000000000000000000000000000000000000"}'
4. **Querying Token Balances:** Calculate the memory address for a specific wallet:
      ```
      export SLOT_32=$(cast --to-bytes32 0)
      export ACCOUNT_32=$(cast --to-uint256 0x5ee5bf7ae06d1be5997a1a72006fe6c607ec6de8)
      echo $ACCOUNT_32
      export ACCOUNT_NO_PREFIX="${ACCOUNT_32#0x}"
      export SLOT_NO_PREFIX="${SLOT_32#0x}"
      export KEY="0x${ACCOUNT_NO_PREFIX}${SLOT_NO_PREFIX}"
      echo $KEY
      export MEM_ADDR=$(cast keccak $KEY)
      echo $MEM_ADDR
      curl http://localhost:3000/storage_at -X POST -H 'Content-Type: application/json' --data "{\"addr\": \"$MEM_ADDR\"}"
    ```
    In order to calculate the memory address for a given token wallet, you need to compute the memory address that corresponds to that token address. You need to know the slot of the addr -> mapping, and the address and then calculate kekkac(addr . slot). For instance, for wbtc, the slot is 0 and let's see how to get the balance for wallet address `0x0000000000000000000000005ee5bf7ae06d1be5997a1a72006fe6c607ec6de8` (you need cast installed here)
     >**Note**: Verify response validity by ensuring the block hash and TDX attestation are correct.
    
5. **Syncing to the Latest Block:** Request a sync to the most recent Ethereum block:
    ```
    curl http://localhost:3000/update -v
---
## Changing the Default Token
The default token contract is WBTC. To change this, update the `WBTCMemoryUpdates` handler to the specific contract's handler. Refer to [obliviouslabs/verified-erc20-state](https://github.com/obliviouslabs/verified_contract_state) for guidance.
