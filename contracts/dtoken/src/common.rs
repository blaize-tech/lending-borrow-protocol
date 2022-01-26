use near_sdk::Balance;
use crate::*;

#[near_bindgen]
impl Contract {

    pub fn get_exchange_rate(&self, underlying_balance: Balance) -> Balance {
        let mut exchange_rate: u128;
        exchange_rate = self.initial_exchange_rate;
        if self.token.total_supply > 0 {
            exchange_rate = (underlying_balance + self.total_borrows - self.total_supplies)
                / self.token.total_supply;
        }
        return exchange_rate;
    }

    pub fn get_total_supplies(&self) -> Balance {
        return self.total_supplies;
    }

    pub fn set_total_supplies(&mut self, amount: Balance) -> Balance {
        self.total_supplies = amount;
        return self.get_total_supplies();
    }

    pub fn get_total_borrows(&self) -> Balance {
        return self.total_borrows;
    }

    pub fn set_total_borrows(&mut self, amount: Balance) -> Balance {
        self.total_supplies = amount;
        return self.get_total_borrows();
    }

}
