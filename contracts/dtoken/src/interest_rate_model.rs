use crate::*;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InterestRateModel {
    kink: Balance,
    multiplier_per_block: Balance,
    base_rate_per_block: Balance,
    jump_multiplier_per_block: Balance,
}

#[near_bindgen]
impl InterestRateModel{
    pub fn get_kink(&self) -> Balance{
        return self.kink;
    }

    pub fn get_multiplier_per_block(&self) -> Balance{
        return self.multiplier_per_block;
    }

    pub fn get_base_rate_per_block(&self) -> Balance{
        return self.base_rate_per_block;
    }

    pub fn get_jump_multiplier_per_block(&self) -> Balance{
        return self.jump_multiplier_per_block;
    }
}

#[near_bindgen]
impl InterestRateModel{
    pub fn get_with_ratio_decimals(value: f32) -> Balance{
        return (value * RATIO_DECIMALS as f32) as Balance;
    }
}

impl Default for InterestRateModel{
    fn default()->Self{
        Self{
            kink: InterestRateModel::get_with_ratio_decimals(1.0),
            base_rate_per_block: InterestRateModel::get_with_ratio_decimals(1.0),
            multiplier_per_block: InterestRateModel::get_with_ratio_decimals(1.0),
            jump_multiplier_per_block: InterestRateModel::get_with_ratio_decimals(1.0),
        }
    }
}