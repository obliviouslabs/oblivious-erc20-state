# Oblivious ERC20 State

This project shows an app that uses [https://www.github.com/obliviouslabs/ordb](obliviouslabs/ordb) and [https://www.github.com/obliviouslabs/oblivious-erc20-state](obliviouslabs/oblivious-erc20-state) to access ierc20 state obliviously inside of TDX.

 
# Components:

+ this folder - http api server, tappd attestation generation, dapp docker image 
+ obliviouslabs/ordb - oblivious database.
+ obliviouslabs/verified-erc20-state - code to fetch ethereum state from an untrusted geth compatible server.


# How to run

First, create a .env file with the any geth compatible api server url, for instance:

```
GETH_URL=https://eth-mainnet.g.alchemy.com/v2/your_api_key
```

Then, make sure you have tappd running on the machine. For testing purposes you can use [https://github.com/Leechael/tappd-simulator](Leechael/tappd-simulator), or just comment out the line about tappd in the Dockerfile (in which case all attestations will be empty).

Then, just build and deploy the docker-compose:

```
  docker build -t oblivious_erc20_state .
  docker-compose up
```

The server will start running, listening on port 3000. 

You can then issue requests. There are two api, the unquoted api, that doesn't generate tdx quotes, and the quoted api, under /quoted (check src/main.rs)

Example status request:
```
  curl http://localhost:3000/quoted/status -v
```

Example address check request (zero address):
```
curl http://localhost:3000/storage_at -X POST -H 'Content-Type: application/json' --data '{"addr": "0000000000000000000000000000000000000000000000000000000000000000"}'
```

Same request, using the quoted api:
```
curl http://localhost:3000/storage_at -X POST -H 'Content-Type: application/json' --data '{"addr": "0000000000000000000000000000000000000000000000000000000000000000"}'
```

In order to calculate the memory address for a given token wallet, you need to compute the memory address that corresponds to that token address. You need to know the slot of the addr -> mapping, and the address and then calculate kekkac(addr . slot). For instance, for wbtc, the slot is 0 and let's see how to get the balance for wallet address '0x0000000000000000000000005ee5bf7ae06d1be5997a1a72006fe6c607ec6de8' (you need [https://github.com/foundry-rs/foundry](cast) installed):
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


In order to trust that a response is valid, the user should make sure the block hash is correct and the tdx attestation is valid.

After starting, the server doesn't update automatically to the most recent block. To request to sync to the most recent ethereum block, you should make a call to /update_handler:
```
  curl http://localhost:3000/update -v
```


# Change used token

By default the chosen contract is wbtc. To change the contract, change the WBTCMemoryUpdates with the update handler for the specific contract, note that the storage slot might also change when you calculate the memory address of a given account. You can take a look at obliviouslabs/verified-erc20-state to verify this and get storage slots for some other contracts.

