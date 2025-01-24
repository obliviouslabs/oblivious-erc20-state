# Oblivious ERC20 State

This project shows an app that uses [https://www.github.com/obliviouslabs/ordb](obliviouslabs/ordb) and [https://www.github.com/xtrm0/oblivious-erc20-state](xtrm0/oblivious-erc20-state) to access ierc20 state obliviously inside of TDX.

 
# Components:

+ this folder - http api server, tappd attestation generation, dapp docker image 
+ obliviouslabs/ordb - oblivious database.
+ obliviouslabs/verified-erc20-state - code to fetch ethereum state from an untrusted geth compatible server.


# How to run

First, create a .env file with the any geth compatible api server url, for instance:

```
GETH_URL=https://eth-mainnet.g.alchemy.com/v2/your_api_key
```

Just build and deploy the docker-compose:

```
  ./build_docker_image.sh
  docker-compose up
```

The server will start running, listening on port 3000. 

You can then issue requests. There are two api, the unquoted api, that doesn't generate tdx quotes, and the quoted api, under /quoted (check src/main.rs)

Example status request:
```
  curl http://localhost:3000/quoted/status -v
```

Example address check request:
```
curl http://localhost:3000/quoted/storage_at -X POST -H 'Content-Type: application/json' --data '{"addr": "0000000000000000000000000000000000000000000000000000000000000000"}'
```
In order to trust that a response is valid, the user should make sure the block hash is correct and the tdx attestation is valid.

After starting, the server doesn't update automatically to the most recent block. To request to sync to the most recent ethereum block, you should make a call to /update_handler:
```
  curl http://localhost:3000/update -v
```


# Change used token

By default the chosen contract is wbtc. To change the contract, change the WBTCMemoryUpdates with the update handler for the specific contract. You can take a look at obliviouslabs/verified-erc20-state to verify this.