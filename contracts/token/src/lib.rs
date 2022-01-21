use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedMap};
use near_sdk::{env, near_bindgen, AccountId, BorshStorageKey};

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Borrows,
}

pub type TokenAmount = u128;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    // Account Id -> Token's amount
    pub borrows: UnorderedMap<AccountId, TokenAmount>
}

impl Default for Contract {
    fn default() -> Self {
        env::panic_str("Token contract should be initialized before usage")
    }
}

#[near_bindgen]
impl Contract {
    /// Initializes the contract with the given config. Needs to be called once.
    #[init]
    pub fn new() -> Self {
        Self {
            borrows: UnorderedMap::new(StorageKey::Borrows),
        }
    }
}

// use the attribute below for unit tests
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{get_logs, VMContextBuilder};
    use near_sdk::{testing_env, AccountId};

    // part of writing unit tests is setting up a mock context
    // provide a `predecessor` here, it'll modify the default context
    fn get_context(predecessor: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
    }

    // TESTS HERE
}
