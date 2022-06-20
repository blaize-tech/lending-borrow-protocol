# login
# near login

# build & test
./build.sh && ./test.sh


# clean up previous deployment
near delete weth_beta.nearlend.testnet nearlend.testnet
near delete dweth_beta.nearlend.testnet nearlend.testnet

near delete wnear_beta.nearlend.testnet nearlend.testnet
near delete dwnear_beta.nearlend.testnet nearlend.testnet

near delete usdt_beta.nearlend.testnet nearlend.testnet
near delete dusdt_beta.nearlend.testnet nearlend.testnet

near delete usdc_beta.nearlend.testnet nearlend.testnet
near delete dusdc_beta.nearlend.testnet nearlend.testnet

near delete controller_beta.nearlend.testnet nearlend.testnet


# create underlying tokens and markets
near create-account weth_beta.nearlend.testnet --masterAccount nearlend.testnet --initialBalance 3
near create-account dweth_beta.nearlend.testnet --masterAccount nearlend.testnet --initialBalance 5

near create-account wnear_beta.nearlend.testnet --masterAccount nearlend.testnet --initialBalance 3
near create-account dwnear_beta.nearlend.testnet --masterAccount nearlend.testnet --initialBalance 5

near create-account usdt_beta.nearlend.testnet --masterAccount nearlend.testnet --initialBalance 3
near create-account dusdt_beta.nearlend.testnet --masterAccount nearlend.testnet --initialBalance 5

near create-account usdc_beta.nearlend.testnet --masterAccount nearlend.testnet --initialBalance 3
near create-account dusdc_beta.nearlend.testnet --masterAccount nearlend.testnet --initialBalance 5


# create controller
near create-account controller_beta.nearlend.testnet --masterAccount nearlend.testnet --initialBalance 10


# deploy underlyings
near deploy weth_beta.nearlend.testnet \
    --wasmFile ./contracts/target/wasm32-unknown-unknown/release/test_utoken.wasm \
    --initFunction 'new_default_meta' \
    --initArgs '{"owner_id": "nearlend.testnet", "name": "Wrapped Ethereum", "symbol": "WETH", "total_supply": "1000000000"}'
near deploy wnear_beta.nearlend.testnet \
    --wasmFile ./contracts/target/wasm32-unknown-unknown/release/test_utoken.wasm \
    --initFunction 'new_default_meta' \
    --initArgs '{"owner_id": "nearlend.testnet", "name": "Wrapped Near", "symbol": "WNEAR", "total_supply": "1000000000"}'
near deploy usdt_beta.nearlend.testnet \
    --wasmFile ./contracts/target/wasm32-unknown-unknown/release/test_utoken.wasm \
    --initFunction 'new_default_meta' \
    --initArgs '{"owner_id": "nearlend.testnet", "name": "Tether", "symbol": "USDT", "total_supply": "1000000000"}'
near deploy usdc_beta.nearlend.testnet \
    --wasmFile ./contracts/target/wasm32-unknown-unknown/release/test_utoken.wasm \
    --initFunction 'new_default_meta' \
    --initArgs '{"owner_id": "nearlend.testnet", "name": "USD Coin", "symbol": "USDC", "total_supply": "1000000000"}'


# deploy markets
near deploy dweth_beta.nearlend.testnet \
    --wasmFile ./contracts/target/wasm32-unknown-unknown/release/dtoken.wasm \
    --initFunction 'new_with_config' \
    --initArgs '{
        "owner_id":"nearlend.testnet",
        "underlying_token_id":"weth_beta.nearlend.testnet",
        "controller_account_id":"controller_beta.nearlend.testnet",
        "initial_exchange_rate":"10000000000",
        "interest_rate_model":{
            "kink":"6500000000",
            "multiplier_per_block":"628",
            "base_rate_per_block":"0",
            "jump_multiplier_per_block":"761",
            "reserve_factor":"1000000000"
        }
    }'
near deploy dwnear_beta.nearlend.testnet \
    --wasmFile ./contracts/target/wasm32-unknown-unknown/release/dtoken.wasm \
    --initFunction 'new_with_config' \
    --initArgs '{
        "owner_id":"nearlend.testnet",
        "underlying_token_id":"wnear_beta.nearlend.testnet",
        "controller_account_id":"controller_beta.nearlend.testnet",
        "initial_exchange_rate":"10000000000",
        "interest_rate_model":{
            "kink":"6500000000",
            "multiplier_per_block":"628",
            "base_rate_per_block":"0",
            "jump_multiplier_per_block":"761",
            "reserve_factor":"1000000000"
        }
    }'
near deploy dusdt_beta.nearlend.testnet \
    --wasmFile ./contracts/target/wasm32-unknown-unknown/release/dtoken.wasm \
    --initFunction 'new_with_config' \
    --initArgs '{
       "owner_id":"nearlend.testnet",
       "underlying_token_id":"usdt_beta.nearlend.testnet",
       "controller_account_id":"controller_beta.nearlend.testnet",
        "initial_exchange_rate":"10000000000",
        "interest_rate_model":{
           "kink":"8000000000",
           "multiplier_per_block":"685",
           "base_rate_per_block":"0",
           "jump_multiplier_per_block":"666",
           "reserve_factor":"1000000000"
        }
    }'
near deploy dusdc_beta.nearlend.testnet \
    --wasmFile ./contracts/target/wasm32-unknown-unknown/release/dtoken.wasm \
    --initFunction 'new_with_config' \
    --initArgs '{
       "owner_id":"nearlend.testnet",
       "underlying_token_id":"usdc_beta.nearlend.testnet",
       "controller_account_id":"controller_beta.nearlend.testnet",
        "initial_exchange_rate":"10000000000",
        "interest_rate_model":{
           "kink":"8000000000",
           "multiplier_per_block":"685",
           "base_rate_per_block":"0",
           "jump_multiplier_per_block":"666",
           "reserve_factor":"1000000000"
        }
    }'

# deploy controller
near deploy controller_beta.nearlend.testnet \
    --wasmFile ./contracts/target/wasm32-unknown-unknown/release/controller.wasm \
    --initFunction 'new_with_config' \
    --initArgs '{
        "owner_id":"nearlend.testnet",
        "oracle_account_id":"oracle_beta.nearlend.testnet"
    }'


# fund dweth_beta.nearlend.testnet
near call weth_beta.nearlend.testnet mint '{"account_id": "dweth_beta.nearlend.testnet", "amount": "1"}' --accountId nearlend.testnet
# fund dwnear_beta.nearlend.testnet
near call wnear_beta.nearlend.testnet mint '{"account_id": "dwnear_beta.nearlend.testnet", "amount": "1"}' --accountId nearlend.testnet
# fund dweth_beta.nearlend.testnet
near call usdt_beta.nearlend.testnet mint '{"account_id": "dusdt_beta.nearlend.testnet", "amount": "1"}' --accountId nearlend.testnet
near call usdc_beta.nearlend.testnet mint '{"account_id": "dusdc_beta.nearlend.testnet", "amount": "1"}' --accountId nearlend.testnet

# register market
near call controller_beta.nearlend.testnet add_market '{"asset_id": "weth_beta.nearlend.testnet", "dtoken": "dweth_beta.nearlend.testnet", "ticker_id": "weth"}' --accountId nearlend.testnet
near call controller_beta.nearlend.testnet add_market '{"asset_id": "wnear_beta.nearlend.testnet", "dtoken": "dwnear_beta.nearlend.testnet", "ticker_id": "wnear"}' --accountId nearlend.testnet
near call controller_beta.nearlend.testnet add_market '{"asset_id": "usdt_beta.nearlend.testnet", "dtoken": "dusdt_beta.nearlend.testnet", "ticker_id": "usdt"}' --accountId nearlend.testnet
near call controller_beta.nearlend.testnet add_market '{"asset_id": "usdc_beta.nearlend.testnet", "dtoken": "dusdc_beta.nearlend.testnet", "ticker_id": "usdc"}' --accountId nearlend.testnet

near view controller_beta.nearlend.testnet view_markets '{}' --accountId controller_beta.nearlend.testnet

near view controller_beta.nearlend.testnet view_prices '{ "dtokens": ["dwnear_beta.nearlend.testnet", "dweth_beta.nearlend.testnet", "dusdt_beta.nearlend.testnet", "dusdc_beta.nearlend.testnet"] }' --accountId controller_beta.nearlend.testnet
