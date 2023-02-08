use crate::big_decimal::{BigDecimal, WRatio};
use crate::*;
use near_sdk::env::signer_account_id;
use near_sdk::Gas;

#[near_bindgen]
impl Contract {
    pub fn view_order(
        &self,
        account_id: AccountId,
        order_id: U128,
        borrow_rate_ratio: WRatio,
    ) -> OrderView {
        let orders = self.orders.get(&account_id).unwrap_or_else(|| {
            panic!("Orders for account: {account_id} not found");
        });

        let order = orders
            .get(&(order_id.0 as u64))
            .unwrap_or_else(|| {
                panic!("Order with id: {} not found", order_id.0);
            })
            .clone();

        let swap_fee = self.get_swap_fee(&order);

        let borrow_fee = WBigDecimal::from(
            BigDecimal::from(borrow_rate_ratio)
                * BigDecimal::from(U128(env::block_height() as u128 - order.block as u128)),
        );

        OrderView {
            order_id,
            status: order.status,
            order_type: order.order_type,
            amount: U128(order.amount),
            sell_token: order.sell_token,
            sell_token_price: WBalance::from(order.sell_token_price.value),
            buy_token: order.buy_token,
            buy_token_price: WBalance::from(order.buy_token_price.value),
            leverage: WBigDecimal::from(order.leverage),
            borrow_fee,
            liquidation_price: self.calculate_liquidation_price(
                U128(order.amount),
                WBigDecimal::from(order.sell_token_price.value),
                WBigDecimal::from(order.buy_token_price.value),
                WBigDecimal::from(order.leverage),
                borrow_fee,
                swap_fee,
            ),
            lpt_id: order.lpt_id,
        }
    }

    pub fn calculate_pnl(
        &self,
        account_id: AccountId,
        order_id: U128,
        data: Option<MarketData>,
    ) -> PnLView {
        let orders = self.orders.get(&account_id).unwrap_or_else(|| {
            panic!("Orders for account: {account_id} not found");
        });

        let order = orders
            .get(&(order_id.0 as u64))
            .unwrap_or_else(|| {
                panic!("Order with id: {} not found", order_id.0);
            })
            .clone();

        let swap_fee = self.get_swap_fee(&order);

        let buy_amount = order.leverage / BigDecimal::from(10_u128.pow(24))
            * BigDecimal::from(order.amount)
            / order.buy_token_price.value;

        let borrow_amount = BigDecimal::from(U128(order.amount))
            * (order.leverage - BigDecimal::from(10u128.pow(24)));

        let mut borrow_fee = BigDecimal::zero();
        #[allow(clippy::unnecessary_unwrap)]
        if data.is_some() && (order.leverage > BigDecimal::one()) {
            borrow_fee = borrow_amount * BigDecimal::from(data.unwrap().borrow_rate_ratio);
        } // fee by blocks count
          //* BigDecimal::from(block_height() - order.block);

        let expect_amount = buy_amount * order.sell_token_price.value
            - borrow_amount
            - borrow_fee
            - borrow_amount * BigDecimal::from(swap_fee);

        let pnlv: PnLView = if expect_amount.round_u128() > order.amount {
            let lenpnl = (expect_amount
                - BigDecimal::from(order.amount)
                - (BigDecimal::from(order.amount)
                    * BigDecimal::from(self.protocol_fee / 10_u128.pow(24))))
            .round_u128();

            PnLView {
                is_profit: true,
                amount: U128(lenpnl),
            }
        } else {
            let lenpnl = (BigDecimal::from(order.amount) - expect_amount).round_u128();

            PnLView {
                is_profit: false,
                amount: U128(lenpnl),
            }
        };

        pnlv
    }

    pub fn view_orders(
        &self,
        account_id: AccountId,
        sell_token: AccountId,
        buy_token: AccountId,
        borrow_rate_ratio: WRatio,
    ) -> Vec<OrderView> {
        let orders = self.orders.get(&account_id).unwrap_or_default();
        let result = orders
            .iter()
            .filter_map(|(id, order)| {
                match order.sell_token == sell_token && order.buy_token == buy_token {
                    true => {
                        let swap_fee = self.get_swap_fee(order);

                        let borrow_fee = WBigDecimal::from(
                            BigDecimal::from(borrow_rate_ratio)
                                * BigDecimal::from(U128(
                                    env::block_height() as u128 - order.block as u128,
                                )),
                        );

                        Some(OrderView {
                            order_id: U128(*id as u128),
                            status: order.status.clone(),
                            order_type: order.order_type.clone(),
                            amount: U128(order.amount),
                            sell_token: order.sell_token.clone(),
                            sell_token_price: WBigDecimal::from(order.sell_token_price.value),
                            buy_token: order.buy_token.clone(),
                            buy_token_price: WBigDecimal::from(order.buy_token_price.value),
                            leverage: WBigDecimal::from(order.leverage),
                            borrow_fee,
                            liquidation_price: self.calculate_liquidation_price(
                                U128(order.amount),
                                WBigDecimal::from(order.sell_token_price.value),
                                WBigDecimal::from(order.buy_token_price.value),
                                WBigDecimal::from(order.leverage),
                                borrow_fee,
                                swap_fee,
                            ),
                            lpt_id: order.lpt_id.clone(),
                        })
                    }
                    false => None,
                }
            })
            .collect::<Vec<OrderView>>();
        result
    }

    pub fn view_pair(&self, sell_token: &AccountId, buy_token: &AccountId) -> TradePair {
        self.supported_markets
            .get(&(sell_token.clone(), buy_token.clone()))
            .unwrap()
    }

    pub fn view_supported_pairs(&self) -> Vec<TradePair> {
        let pairs = self
            .supported_markets
            .iter()
            .map(|(_, trade_pair)| trade_pair)
            .collect::<Vec<TradePair>>();

        pairs
    }

    /// Returns the balance of the given account on certain token. If the account doesn't exist will return `"0"`.
    pub fn balance_of(&self, account_id: AccountId, token: AccountId) -> WBalance {
        match self.balances.get(&account_id) {
            None => WBalance::from(0_u128),
            Some(user_balance_per_token) => {
                WBalance::from(*user_balance_per_token.get(&token).unwrap_or(&0_u128))
            }
        }
    }

    /// Returns price of the given token. If the token is not registered, will return `"0"`.
    pub fn view_price(&self, token_id: AccountId) -> Price {
        self.prices.get(&token_id).unwrap_or_else(|| {
            panic!("Price for token: {token_id} not found");
        })
    }

    pub fn cancel_order_view(
        &self,
        account_id: AccountId,
        order_id: U128,
        market_data: MarketData,
    ) -> CancelOrderView {
        let orders = self.orders.get(&account_id).unwrap_or_else(|| {
            panic!("Orders for account: {account_id} not found");
        });

        let order = orders.get(&(order_id.0 as u64)).unwrap_or_else(|| {
            panic!("Order with id: {} not found", order_id.0);
        });

        let buy_token =
            BigDecimal::from(U128(order.amount)) * order.leverage * order.sell_token_price.value
                / order.buy_token_price.value;

        let sell_token = BigDecimal::from(U128(order.amount)) * order.leverage;

        let open_price = order.buy_token_price.clone();

        let close_price = self.get_price(&order.buy_token);

        let calc_pnl = self.calculate_pnl(account_id, order_id, Some(market_data));

        CancelOrderView {
            buy_token_amount: WRatio::from(buy_token),
            sell_token_amount: WRatio::from(sell_token),
            open_price: WRatio::from(open_price.value),
            close_price: WRatio::from(close_price),
            pnl: calc_pnl,
        }
    }

    pub fn view_liquidation_threshold(&self) -> U128 {
        U128(self.liquidation_threshold)
    }

    pub fn calculate_liquidation_price(
        &self,
        sell_token_amount: U128,
        sell_token_price: U128,
        buy_token_price: U128,
        leverage: U128,
        borrow_fee: U128,
        swap_fee: U128,
    ) -> WBigDecimal {
        require!(
            sell_token_price != U128::from(0),
            "Sell token price cannot be zero"
        );

        require!(
            buy_token_price != U128::from(0),
            "Buy token price cannot be zero"
        );

        let collateral_usd =
            BigDecimal::from(sell_token_amount) * BigDecimal::from(sell_token_price);
        let position_amount_usd = collateral_usd * BigDecimal::from(leverage);
        let borrow_amount = collateral_usd * (BigDecimal::from(leverage) - BigDecimal::one());
        let buy_amount = position_amount_usd / BigDecimal::from(buy_token_price);

        let liquidation_price = (position_amount_usd - self.volatility_rate * collateral_usd
            + borrow_amount * BigDecimal::from(borrow_fee)
            + position_amount_usd * BigDecimal::from(swap_fee))
            / buy_amount;

        liquidation_price.into()
    }

    /// returns const gas amount required for executing orders: 50 TGas
    pub fn view_gas_for_execution(&self) -> Balance {
        Gas::ONE_TERA.0 as Balance * 50u128
    }

    pub fn view_max_position_amount(&self) -> U128 {
        U128(self.max_order_amount)
    }

    pub fn get_total_pending_orders_per_pair(&self, pair_id: &PairId) -> U128 {
        let total = self
            .orders_per_pair_view
            .get(pair_id)
            .unwrap_or_else(|| {
                panic!(
                    "Total pending orders for pair {} | {} not found",
                    pair_id.0, pair_id.1
                )
            })
            .len();

        U128(total as u128)
    }

    pub fn get_pending_orders(
        &self,
        pair_id: &PairId,
        user_per_page: U128,
        page: U128,
    ) -> PendingOrders {
        let orders = self.orders_per_pair_view.get(pair_id).unwrap_or_default();
        let mut pending_orders = orders
            .iter()
            .filter_map(|(id, order)| match order.status == OrderStatus::Pending {
                true => Some((*id, order.clone())),
                false => None,
            })
            .collect::<Vec<(u64, Order)>>();

        pending_orders.sort_by(|a, b| a.0.cmp(&b.0));

        let total = U128(pending_orders.len() as u128);

        let sort_pending_orders = pending_orders
            .into_iter()
            .skip((user_per_page.0 * page.0 - user_per_page.0) as usize)
            .take(user_per_page.0 as usize)
            .collect();

        PendingOrders {
            data: sort_pending_orders,
            page,
            total,
        }
    }

    pub fn view_pair_tokens_decimals(
        &self,
        sell_token: &AccountId,
        buy_token: &AccountId,
    ) -> (u8, u8) {
        let pair_id = &(sell_token.clone(), buy_token.clone());
        let pair = self.supported_markets.get(pair_id).unwrap_or_else(|| {
            panic!(
                "Sell and Buy token decimals for pair {} | {} not found",
                pair_id.0, pair_id.1
            )
        });
        (pair.sell_token_decimals, pair.buy_token_decimals)
    }

    pub fn view_token_decimals(&self, token: &AccountId) -> u8 {
        let pair_id = self
            .supported_markets
            .keys()
            .find(|pair| pair.0 == token.clone() || pair.1 == token.clone());
        if let Some((sell_token, buy_token)) = pair_id {
            let (sell_token_decimals, buy_token_decimals) =
                self.view_pair_tokens_decimals(&sell_token, &buy_token);
            if token == &sell_token {
                sell_token_decimals
            } else {
                buy_token_decimals
            }
        } else {
            panic!("Token is not supported");
        }
    }

    #[allow(unused_variables)]
    pub fn view_pending_limit_orders_by_user(&self, account_id: AccountId) -> Vec<LimitOrderView> {
        let order_type = "Limit".to_string();

        let limit_order_1 = LimitOrderView {
            time_stamp: 1675332394281,
            pair: "USDT/NEAR".to_string(),
            order_type: order_type.clone(),
            side: OrderType::Buy,
            price: WBalance::from(302 * 10_u128.pow(22)), // 3.05 / 1.01 = 3.01980198
            amount: U128(500 * 10_u128.pow(24)),
            filled: 0,
            total: WBalance::from(505 * 10_u128.pow(24)), //500 * 1.01 = 505
        };

        let limit_order_2 = LimitOrderView {
            time_stamp: 1675336439706,
            pair: "USDT/NEAR".to_string(),
            order_type: order_type.clone(),
            side: OrderType::Buy,
            price: WBalance::from(302 * 10_u128.pow(22)), // 3.05 / 1.01 = 3.01980198
            amount: U128(1000 * 10_u128.pow(24)),
            filled: 0,
            total: WBalance::from(1010 * 10_u128.pow(24)), //1000 * 1.01 = 1010
        };

        let limit_order_3 = LimitOrderView {
            time_stamp: 1675335439706,
            pair: "NEAR/USDT".to_string(),
            order_type,
            side: OrderType::Buy,
            price: WBalance::from(33 * 10_u128.pow(22)), // 1.01 / 3.05 = 0.331147541
            amount: U128(1000 * 10_u128.pow(24)),
            filled: 0,
            total: WBalance::from(3050 * 10_u128.pow(24)), //1000 * 3.05 = 3050
        };

        vec![limit_order_1, limit_order_2, limit_order_3]
    }

    #[allow(unused_variables)]
    pub fn view_pending_limit_orders_by_user_by_pair(
        &self,
        account_id: AccountId,
        sell_token: AccountId,
        buy_token: AccountId,
    ) -> Vec<LimitOrderView> {
        let order_type = "Limit".to_string();

        let limit_order_1 = LimitOrderView {
            time_stamp: 1675332394281,
            pair: "USDT/NEAR".to_string(),
            order_type: order_type.clone(),
            side: OrderType::Buy,
            price: WBalance::from(302 * 10_u128.pow(22)), // 3.05 / 1.01 = 3.01980198
            amount: U128(500 * 10_u128.pow(24)),
            filled: 0,
            total: WBalance::from(505 * 10_u128.pow(24)), //500 * 1.01 = 505
        };

        let limit_order_2 = LimitOrderView {
            time_stamp: 1675336439706,
            pair: "USDT/NEAR".to_string(),
            order_type,
            side: OrderType::Buy,
            price: WBalance::from(302 * 10_u128.pow(22)), // 3.05 / 1.01 = 3.01980198
            amount: U128(1000 * 10_u128.pow(24)),
            filled: 0,
            total: WBalance::from(1010 * 10_u128.pow(24)), //1000 * 1.01 = 1010
        };

        vec![limit_order_1, limit_order_2]
    }

    pub fn take_profit_order_view(&self, order_id: U128) -> TakeProfitOrderView {
        require!(
            Some(signer_account_id()) == self.get_account_by(order_id.0),
            "You have no access for this order."
        );

        let order = self
            .take_profit_orders
            .get(&(order_id.0 as u64))
            .unwrap_or_else(|| panic!("Take profit order for: {} not found", order_id.0));

        TakeProfitOrderView {
            order_id,
            take_profit_price: WRatio::from(order.buy_token_price.value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use near_sdk::test_utils::test_env::{alice, bob};
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, VMContext};

    fn get_context(is_view: bool) -> VMContext {
        VMContextBuilder::new()
            .current_account_id("margin.nearland.testnet".parse().unwrap())
            .signer_account_id(alice())
            .predecessor_account_id("usdt_market.qa.nearland.testnet".parse().unwrap())
            .block_index(103930916)
            .block_timestamp(1)
            .is_view(is_view)
            .build()
    }

    #[test]
    fn test_get_pending_orders() {
        let context = get_context(false);
        testing_env!(context);

        let mut contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );

        let pair_id: PairId = (
            "usdt.qa.v1.nearlend.testnet".parse().unwrap(),
            "wnear.qa.v1.nearlend.testnet".parse().unwrap(),
        );

        let pair_data = TradePair {
            sell_ticker_id: "USDT".to_string(),
            sell_token: "usdt.qa.v1.nearlend.testnet".parse().unwrap(),
            sell_token_decimals: 24,
            sell_token_market: "usdt_market.qa.v1.nearlend.testnet".parse().unwrap(),
            buy_ticker_id: "WNEAR".to_string(),
            buy_token: "wnear.qa.v1.nearlend.testnet".parse().unwrap(),
            buy_token_decimals: 24,
            pool_id: "usdt.qa.v1.nearlend.testnet|wnear.qa.v1.nearlend.testnet|2000".to_string(),
            max_leverage: U128(25 * 10_u128.pow(23)),
            swap_fee: U128(3 * 10_u128.pow(20)),
        };

        let market_data = MarketData {
            underlying_token: AccountId::new_unchecked("usdt.fakes.testnet".to_string()),
            underlying_token_decimals: 6,
            total_supplies: U128(60000000000000000000000000000),
            total_borrows: U128(25010000000000000000000000000),
            total_reserves: U128(1000176731435219096024128768),
            exchange_rate_ratio: U128(1000277139994639276176632),
            interest_rate_ratio: U128(261670051778601),
            borrow_rate_ratio: U128(634273735391536),
        };

        contract.update_or_insert_price(
            "usdt.qa.v1.nearlend.testnet".parse().unwrap(),
            Price {
                ticker_id: "USDT".to_string(),
                value: BigDecimal::from(1.01),
            },
        );
        contract.update_or_insert_price(
            "wnear.qa.v1.nearlend.testnet".parse().unwrap(),
            Price {
                ticker_id: "WNEAR".to_string(),
                value: BigDecimal::from(3.05),
            },
        );

        contract.add_pair(pair_data);

        contract.set_balance(&alice(), &pair_id.0, 10_u128.pow(30));

        let price_impact = U128(1);
        let order_as_string = "{\"status\":\"Pending\",\"order_type\":\"Buy\",\"amount\":1000000000000000000000000000,\"sell_token\":\"usdt.qa.v1.nearlend.testnet\",\"buy_token\":\"wnear.qa.v1.nearlend.testnet\",\"leverage\":\"1000000000000000000000000\",\"sell_token_price\":{\"ticker_id\":\"USDT\",\"value\":\"1.01\"},\"buy_token_price\":{\"ticker_id\":\"WNEAR\",\"value\":\"3.05\"},\"block\":103930910,\"lpt_id\":\"usdt.qa.v1.nearlend.testnet|wnear.qa.v1.nearlend.testnet|2000#540\"}".to_string();
        let order: Order = near_sdk::serde_json::from_str(order_as_string.as_str()).unwrap();

        for count in 0..9 {
            if count < 6 {
                contract.imitation_add_liquidity_callback(order.clone());
            } else {
                contract.final_order_cancel(
                    U128(count as u128 - 5),
                    order.clone(),
                    price_impact,
                    Some(market_data.clone()),
                );
            }
        }

        let pending_orders_par_1st_page = contract.get_pending_orders(&pair_id, U128(10), U128(1));
        let order_id_with_pending_status = pending_orders_par_1st_page
            .data
            .iter()
            .map(|(order_id, _)| *order_id)
            .collect::<Vec<u64>>();

        assert_eq!(
            contract.orders_per_pair_view.get(&pair_id).unwrap().len(),
            6_usize
        );
        assert_eq!(pending_orders_par_1st_page.data.len(), 3_usize);
        assert_eq!(
            pending_orders_par_1st_page.data.get(0).unwrap().1.status,
            OrderStatus::Pending
        );
        assert_eq!(pending_orders_par_1st_page.total, U128(3));
        assert_eq!(order_id_with_pending_status, vec![4, 5, 6]);

        let pending_orders_par_2nd_page = contract.get_pending_orders(&pair_id, U128(10), U128(2));

        assert_eq!(pending_orders_par_2nd_page.data.len(), 0_usize);
    }

    #[test]
    fn view_supported_pairs_test() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );
        let pair_data = TradePair {
            sell_ticker_id: "USDt".to_string(),
            sell_token: "usdt.fakes.testnet".parse().unwrap(),
            sell_token_decimals: 24,
            sell_token_market: "usdt_market.develop.v1.omomo-finance.testnet"
                .parse()
                .unwrap(),
            buy_ticker_id: "near".to_string(),
            buy_token: "wrap.testnet".parse().unwrap(),
            buy_token_decimals: 24,
            pool_id: "usdt.fakes.testnet|wrap.testnet|2000".to_string(),
            max_leverage: U128(25 * 10_u128.pow(23)),
            swap_fee: U128(3 * 10_u128.pow(20)),
        };
        contract.add_pair(pair_data.clone());

        let pair_data2 = TradePair {
            sell_ticker_id: "near".to_string(),
            sell_token: "wrap.testnet".parse().unwrap(),
            sell_token_decimals: 24,
            sell_token_market: "wnear_market.develop.v1.omomo-finance.testnet"
                .parse()
                .unwrap(),
            buy_ticker_id: "USDt".to_string(),
            buy_token: "usdt.fakes.testnet".parse().unwrap(),
            buy_token_decimals: 24,
            pool_id: "usdt.fakes.testnet|wrap.testnet|2000".to_string(),
            max_leverage: U128(25 * 10_u128.pow(23)),
            swap_fee: U128(3 * 10_u128.pow(20)),
        };

        contract.add_pair(pair_data2.clone());

        let result = vec![pair_data, pair_data2];
        let pairs = contract.view_supported_pairs();
        assert_eq!(result, pairs);
    }

    #[test]
    fn calculate_pnl_test() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );

        let pair_data = TradePair {
            sell_ticker_id: "USDt".to_string(),
            sell_token: "usdt.fakes.testnet".parse().unwrap(),
            sell_token_decimals: 6,
            sell_token_market: "usdt_market.develop.v1.omomo-finance.testnet"
                .parse()
                .unwrap(),
            buy_ticker_id: "near".to_string(),
            buy_token: "wrap.testnet".parse().unwrap(),
            buy_token_decimals: 24,
            pool_id: "usdt.fakes.testnet|wrap.testnet|2000".to_string(),
            max_leverage: U128(25 * 10_u128.pow(23)),
            swap_fee: U128(3 * 10_u128.pow(20)),
        };
        contract.add_pair(pair_data);

        contract.update_or_insert_price(
            "usdt.fakes.testnet".parse().unwrap(),
            Price {
                ticker_id: "USDt".to_string(),
                value: BigDecimal::from(2.0),
            },
        );
        contract.update_or_insert_price(
            "wrap.testnet".parse().unwrap(),
            Price {
                ticker_id: "near".to_string(),
                value: BigDecimal::from(4.22),
            },
        );

        let order1 = "{\"status\":\"Executed\",\"order_type\":\"Buy\",\"amount\":1500000000000000000000000000,\"sell_token\":\"usdt.fakes.testnet\",\"buy_token\":\"wrap.testnet\",\"leverage\":\"2000000000000000000000000\",\"sell_token_price\":{\"ticker_id\":\"USDT\",\"value\":\"3.3\"},\"buy_token_price\":{\"ticker_id\":\"WNEAR\",\"value\":\"4.59\"},\"block\":1,\"lpt_id\":\"usdt.fakes.testnet|wrap.testnet|2000#132\"}".to_string();
        contract.add_order_from_string(alice(), order1);

        let market_data = MarketData {
            underlying_token: AccountId::new_unchecked("usdt.fakes.testnet".to_string()),
            underlying_token_decimals: 6,
            total_supplies: U128(10_u128.pow(24)),
            total_borrows: U128(10_u128.pow(24)),
            total_reserves: U128(10_u128.pow(24)),
            exchange_rate_ratio: U128(10_u128.pow(24)),
            interest_rate_ratio: U128(10_u128.pow(24)),
            borrow_rate_ratio: U128(5 * 10_u128.pow(22)),
        };
        let pnl = contract.calculate_pnl(alice(), U128(1), Some(market_data));
        assert!(!pnl.is_profit);
        assert_eq!(pnl.amount, U128(918587254901960784313725490));
    }

    #[test]
    fn calculate_pnl_leverage_3_test() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );

        let pair_data = TradePair {
            sell_ticker_id: "USDt".to_string(),
            sell_token: "usdt.fakes.testnet".parse().unwrap(),
            sell_token_decimals: 6,
            sell_token_market: "usdt_market.develop.v1.omomo-finance.testnet"
                .parse()
                .unwrap(),
            buy_ticker_id: "near".to_string(),
            buy_token: "wrap.testnet".parse().unwrap(),
            buy_token_decimals: 24,
            pool_id: "usdt.fakes.testnet|wrap.testnet|2000".to_string(),
            max_leverage: U128(25 * 10_u128.pow(23)),
            swap_fee: U128(3 * 10_u128.pow(20)),
        };
        contract.add_pair(pair_data);

        contract.update_or_insert_price(
            "usdt.fakes.testnet".parse().unwrap(),
            Price {
                ticker_id: "USDt".to_string(),
                value: BigDecimal::from(2.0),
            },
        );
        contract.update_or_insert_price(
            "wrap.testnet".parse().unwrap(),
            Price {
                ticker_id: "near".to_string(),
                value: BigDecimal::from(4.22),
            },
        );

        let order1 = "{\"status\":\"Executed\",\"order_type\":\"Buy\",\"amount\":1500000000000000000000000000,\"sell_token\":\"usdt.fakes.testnet\",\"buy_token\":\"wrap.testnet\",\"leverage\":\"3000000000000000000000000\",\"sell_token_price\":{\"ticker_id\":\"USDT\",\"value\":\"3.3\"},\"buy_token_price\":{\"ticker_id\":\"WNEAR\",\"value\":\"4.59\"},\"block\":1,\"lpt_id\":\"usdt.fakes.testnet|wrap.testnet|2000#132\"}".to_string();
        contract.add_order_from_string(alice(), order1);

        let market_data = MarketData {
            underlying_token: AccountId::new_unchecked("alice.testnet".to_string()),
            underlying_token_decimals: 24,
            total_supplies: U128(10_u128.pow(24)),
            total_borrows: U128(10_u128.pow(24)),
            total_reserves: U128(10_u128.pow(24)),
            exchange_rate_ratio: U128(10_u128.pow(24)),
            interest_rate_ratio: U128(10_u128.pow(24)),
            borrow_rate_ratio: U128(5 * 10_u128.pow(22)),
        };
        let pnl = contract.calculate_pnl(alice(), U128(1), Some(market_data));
        assert!(!pnl.is_profit);
        assert_eq!(pnl.amount, U128(1415605882352941176470588235));
    }

    #[test]
    fn test_calculate_liquidation_leverage_3() {
        let mut contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );

        let pair_data = TradePair {
            sell_ticker_id: "USDt".to_string(),
            sell_token: "usdt.fakes.testnet".parse().unwrap(),
            sell_token_decimals: 6,
            sell_token_market: "usdt_market.develop.v1.omomo-finance.testnet"
                .parse()
                .unwrap(),
            buy_ticker_id: "near".to_string(),
            buy_token: "wrap.testnet".parse().unwrap(),
            buy_token_decimals: 24,
            pool_id: "usdt.fakes.testnet|wrap.testnet|2000".to_string(),
            max_leverage: U128(25 * 10_u128.pow(23)),
            swap_fee: U128(3 * 10_u128.pow(20)),
        };
        contract.add_pair(pair_data.clone());

        let result = contract.calculate_liquidation_price(
            U128(10_u128.pow(27)),
            U128(10_u128.pow(24)),
            U128(10_u128.pow(25)),
            U128(3 * 10_u128.pow(24)),
            U128(5 * 10_u128.pow(22)),
            U128(3 * 10_u128.pow(20)),
        );

        assert_eq!(result, U128(7169666666666666666666666));
    }

    #[test]
    fn test_calculate_liquidation_leverage_1_5() {
        let mut contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );

        let pair_data = TradePair {
            sell_ticker_id: "USDt".to_string(),
            sell_token: "usdt.fakes.testnet".parse().unwrap(),
            sell_token_decimals: 6,
            sell_token_market: "usdt_market.develop.v1.omomo-finance.testnet"
                .parse()
                .unwrap(),
            buy_ticker_id: "near".to_string(),
            buy_token: "wrap.testnet".parse().unwrap(),
            buy_token_decimals: 24,
            pool_id: "usdt.fakes.testnet|wrap.testnet|2000".to_string(),
            max_leverage: U128(25 * 10_u128.pow(23)),
            swap_fee: U128(3 * 10_u128.pow(20)),
        };
        contract.add_pair(pair_data.clone());

        let result = contract.calculate_liquidation_price(
            U128(10_u128.pow(27)),
            U128(10_u128.pow(24)),
            U128(10_u128.pow(25)),
            U128(15 * 10_u128.pow(23)),
            U128(5 * 10_u128.pow(22)),
            U128(3 * 10_u128.pow(20)),
        );

        assert_eq!(result, U128(3836333333333333333333333));
    }

    #[test]
    fn test_view_order() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );

        let pair_data = TradePair {
            sell_ticker_id: "USDt".to_string(),
            sell_token: "usdt.fakes.testnet".parse().unwrap(),
            sell_token_decimals: 6,
            sell_token_market: "usdt_market.develop.v1.omomo-finance.testnet"
                .parse()
                .unwrap(),
            buy_ticker_id: "near".to_string(),
            buy_token: "wrap.testnet".parse().unwrap(),
            buy_token_decimals: 24,
            pool_id: "usdt.fakes.testnet|wrap.testnet|2000".to_string(),
            max_leverage: U128(25 * 10_u128.pow(23)),
            swap_fee: U128(10u128.pow(23)),
        };
        contract.add_pair(pair_data.clone());

        let borrow_rate_ratio = U128(634273735391536);

        let order1 = "{\"status\":\"Pending\",\"order_type\":\"Buy\",\"amount\":1000000000,\"sell_token\":\"usdt.fakes.testnet\",\"buy_token\":\"wrap.testnet\",\"leverage\":\"1\",\"sell_token_price\":{\"ticker_id\":\"USDT\",\"value\":\"1.01\"},\"buy_token_price\":{\"ticker_id\":\"WNEAR\",\"value\":\"3.05\"},\"block\":103930910,\"lpt_id\":\"usdt.fakes.testnet|wrap.testnet|2000#540\"}".to_string();
        contract.add_order_from_string(alice(), order1);

        let order2 = "{\"status\":\"Pending\",\"order_type\":\"Buy\",\"amount\":1000000000,\"sell_token\":\"usdt.fakes.testnet\",\"buy_token\":\"wrap.testnet\",\"leverage\":\"1\",\"sell_token_price\":{\"ticker_id\":\"USDT\",\"value\":\"1.01\"},\"buy_token_price\":{\"ticker_id\":\"WNEAR\",\"value\":\"3.05\"},\"block\":103930910,\"lpt_id\":\"usdt.fakes.testnet|wrap.testnet|2000#541\"}".to_string();
        contract.add_order_from_string(alice(), order2);

        let order_id = U128(1); //order_id for order1

        let block_order = 103930910_u64;

        let borrow_fee = WBigDecimal::from(
            BigDecimal::from(borrow_rate_ratio)
                * BigDecimal::from(U128(env::block_height() as u128 - block_order as u128)),
        );

        let liquidation_price = contract.calculate_liquidation_price(
            U128(10_u128.pow(9)),
            U128(101 * 10_u128.pow(22)),
            U128(305 * 10_u128.pow(22)),
            U128(10_u128.pow(24)),
            borrow_fee,
            pair_data.swap_fee, // hardcore of swap_fee 0.1 % with 10^24 precision
        );

        let result_view_order1 = OrderView {
            order_id: U128(1),
            status: OrderStatus::Pending,
            order_type: OrderType::Buy,
            amount: U128(10_u128.pow(9)),
            sell_token: "usdt.fakes.testnet".parse().unwrap(),
            sell_token_price: U128(101 * 10_u128.pow(22)), // 1.01 with 10^24 precision
            buy_token: "wrap.testnet".parse().unwrap(),
            buy_token_price: U128(305 * 10_u128.pow(22)), // 3.05 with 10^24 precision
            leverage: U128(10_u128.pow(24)),              // 1 with 10^24 precision
            borrow_fee,
            liquidation_price,
            lpt_id: "usdt.fakes.testnet|wrap.testnet|2000#540".to_string(),
        };

        assert_eq!(
            contract.view_order(alice(), order_id, borrow_rate_ratio),
            result_view_order1
        );
    }

    #[test]
    fn test_view_orders() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );

        let pair_data = TradePair {
            sell_ticker_id: "USDt".to_string(),
            sell_token: "usdt.fakes.testnet".parse().unwrap(),
            sell_token_decimals: 6,
            sell_token_market: "usdt_market.develop.v1.omomo-finance.testnet"
                .parse()
                .unwrap(),
            buy_ticker_id: "near".to_string(),
            buy_token: "wrap.testnet".parse().unwrap(),
            buy_token_decimals: 24,
            pool_id: "usdt.fakes.testnet|wrap.testnet|2000".to_string(),
            max_leverage: U128(25 * 10_u128.pow(23)),
            swap_fee: U128(10u128.pow(23)),
        };
        contract.add_pair(pair_data.clone());

        let borrow_rate_ratio = U128(634273735391536);

        let order1 = "{\"status\":\"Pending\",\"order_type\":\"Buy\",\"amount\":1000000000,\"sell_token\":\"usdt.fakes.testnet\",\"buy_token\":\"wrap.testnet\",\"leverage\":\"1\",\"sell_token_price\":{\"ticker_id\":\"USDT\",\"value\":\"1.01\"},\"buy_token_price\":{\"ticker_id\":\"WNEAR\",\"value\":\"3.05\"},\"block\":103930910,\"lpt_id\":\"usdt.fakes.testnet|wrap.testnet|2000#540\"}".to_string();
        contract.add_order_from_string(alice(), order1);

        let order2 = "{\"status\":\"Pending\",\"order_type\":\"Buy\",\"amount\":2000000000,\"sell_token\":\"usdt.fakes.testnet\",\"buy_token\":\"wrap.testnet\",\"leverage\":\"1\",\"sell_token_price\":{\"ticker_id\":\"USDT\",\"value\":\"1.01\"},\"buy_token_price\":{\"ticker_id\":\"WNEAR\",\"value\":\"3.05\"},\"block\":103930911,\"lpt_id\":\"usdt.fakes.testnet|wrap.testnet|2000#541\"}".to_string();
        contract.add_order_from_string(alice(), order2);

        let order3 = "{\"status\":\"Pending\",\"order_type\":\"Buy\",\"amount\":2000000000,\"sell_token\":\"usdt.fakes.testnet\",\"buy_token\":\"wrap.testnet\",\"leverage\":\"1\",\"sell_token_price\":{\"ticker_id\":\"USDT\",\"value\":\"1.01\"},\"buy_token_price\":{\"ticker_id\":\"WNEAR\",\"value\":\"3.05\"},\"block\":103930912,\"lpt_id\":\"usdt.fakes.testnet|wrap.testnet|2000#542\"}".to_string();
        contract.add_order_from_string(bob(), order3);

        let block_order1 = 103930910_u64;
        let block_order2 = 103930911_u64;
        let block_order3 = 103930912_u64;

        let borrow_fee_order1 = WBigDecimal::from(
            BigDecimal::from(borrow_rate_ratio)
                * BigDecimal::from(U128(env::block_height() as u128 - block_order1 as u128)),
        );

        let borrow_fee_order2 = WBigDecimal::from(
            BigDecimal::from(borrow_rate_ratio)
                * BigDecimal::from(U128(env::block_height() as u128 - block_order2 as u128)),
        );

        let borrow_fee_order3 = WBigDecimal::from(
            BigDecimal::from(borrow_rate_ratio)
                * BigDecimal::from(U128(env::block_height() as u128 - block_order3 as u128)),
        );

        let liquidation_price_order1 = contract.calculate_liquidation_price(
            U128(10_u128.pow(9)),
            U128(101 * 10_u128.pow(22)),
            U128(305 * 10_u128.pow(22)),
            U128(10_u128.pow(24)),
            borrow_fee_order1,
            pair_data.swap_fee, // hardcore of swap_fee 0.1 % with 10^24 precision
        );

        let liquidation_price_order2 = contract.calculate_liquidation_price(
            U128(2 * 10_u128.pow(9)),
            U128(101 * 10_u128.pow(22)),
            U128(305 * 10_u128.pow(22)),
            U128(10_u128.pow(24)),
            borrow_fee_order2,
            pair_data.swap_fee, // hardcore of swap_fee 0.1 % with 10^24 precision
        );

        let liquidation_price_order3 = contract.calculate_liquidation_price(
            U128(2 * 10_u128.pow(9)),
            U128(101 * 10_u128.pow(22)),
            U128(305 * 10_u128.pow(22)),
            U128(10_u128.pow(24)),
            borrow_fee_order3,
            pair_data.swap_fee, // hardcore of swap_fee 0.1 % with 10^24 precision
        );

        let result_view_orders_alice = vec![
            OrderView {
                order_id: U128(1),
                status: OrderStatus::Pending,
                order_type: OrderType::Buy,
                amount: U128(10_u128.pow(9)),
                sell_token: "usdt.fakes.testnet".parse().unwrap(),
                sell_token_price: U128(101 * 10_u128.pow(22)), // 1.01 with 10^24 precision
                buy_token: "wrap.testnet".parse().unwrap(),
                buy_token_price: U128(305 * 10_u128.pow(22)), // 3.05 with 10^24 precision
                leverage: U128(10_u128.pow(24)),              // 1 with 10^24 precision
                borrow_fee: borrow_fee_order1,
                liquidation_price: liquidation_price_order1,
                lpt_id: "usdt.fakes.testnet|wrap.testnet|2000#540".to_string(),
            },
            OrderView {
                order_id: U128(2),
                status: OrderStatus::Pending,
                order_type: OrderType::Buy,
                amount: U128(2 * 10_u128.pow(9)),
                sell_token: "usdt.fakes.testnet".parse().unwrap(),
                sell_token_price: U128(101 * 10_u128.pow(22)), // 1.01 with 10^24 precision
                buy_token: "wrap.testnet".parse().unwrap(),
                buy_token_price: U128(305 * 10_u128.pow(22)), // 3.05 with 10^24 precision
                leverage: U128(10_u128.pow(24)),              // 1 with 10^24 precision
                borrow_fee: borrow_fee_order2,
                liquidation_price: liquidation_price_order2,
                lpt_id: "usdt.fakes.testnet|wrap.testnet|2000#541".to_string(),
            },
        ];

        let result_view_orders_bob = vec![OrderView {
            order_id: U128(3),
            status: OrderStatus::Pending,
            order_type: OrderType::Buy,
            amount: U128(2 * 10_u128.pow(9)),
            sell_token: "usdt.fakes.testnet".parse().unwrap(),
            sell_token_price: U128(101 * 10_u128.pow(22)), // 1.01 with 10^24 precision
            buy_token: "wrap.testnet".parse().unwrap(),
            buy_token_price: U128(305 * 10_u128.pow(22)), // 3.05 with 10^24 precision
            leverage: U128(10_u128.pow(24)),              // 1 with 10^24 precision
            borrow_fee: borrow_fee_order3,
            liquidation_price: liquidation_price_order3,
            lpt_id: "usdt.fakes.testnet|wrap.testnet|2000#542".to_string(),
        }];

        let mut view_orders_alice = contract.view_orders(
            alice(),
            pair_data.sell_token.clone(),
            pair_data.buy_token.clone(),
            borrow_rate_ratio,
        );
        view_orders_alice.sort_by(|a, b| a.order_id.cmp(&b.order_id));

        let view_orders_bob = contract.view_orders(
            bob(),
            pair_data.sell_token,
            pair_data.buy_token,
            borrow_rate_ratio,
        );

        assert_eq!(view_orders_alice, result_view_orders_alice);
        assert_eq!(view_orders_bob, result_view_orders_bob);
    }

    #[test]
    fn test_view_pair_tokens_decimals() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );

        let pair_data = TradePair {
            sell_ticker_id: "usdt".to_string(),
            sell_token: "usdt.qa.v1.nearlend.testnet".parse().unwrap(),
            sell_token_decimals: 24,
            sell_token_market: "usdt_market.qa.v1.nearlend.testnet".parse().unwrap(),
            buy_ticker_id: "wnear".to_string(),
            buy_token: "wnear.qa.v1.nearlend.testnet".parse().unwrap(),
            buy_token_decimals: 24,
            pool_id: "usdt.qa.v1.nearlend.testnet|wnear.qa.v1.nearlend.testnet|2000".to_string(),
            max_leverage: U128(25 * 10_u128.pow(23)),
            swap_fee: U128(3 * 10_u128.pow(20)),
        };
        contract.add_pair(pair_data.clone());

        let sell_and_buy_tokens_decimals =
            contract.view_pair_tokens_decimals(&pair_data.sell_token, &pair_data.buy_token);

        assert_eq!(
            sell_and_buy_tokens_decimals,
            (pair_data.sell_token_decimals, pair_data.buy_token_decimals)
        );
    }

    #[test]
    fn view_token_decimals_test() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );

        let pair_data = TradePair {
            sell_ticker_id: "usdt".to_string(),
            sell_token: "usdt.qa.v1.nearlend.testnet".parse().unwrap(),
            sell_token_decimals: 24,
            sell_token_market: "usdt_market.qa.v1.nearlend.testnet".parse().unwrap(),
            buy_ticker_id: "wnear".to_string(),
            buy_token: "wnear.qa.v1.nearlend.testnet".parse().unwrap(),
            buy_token_decimals: 18,
            pool_id: "usdt.qa.v1.nearlend.testnet|wnear.qa.v1.nearlend.testnet|2000".to_string(),
            max_leverage: U128(25 * 10_u128.pow(23)),
            swap_fee: U128(3 * 10_u128.pow(20)),
        };
        contract.add_pair(pair_data.clone());

        let sell_token_decimals = contract.view_token_decimals(&pair_data.sell_token);
        let buy_token_decimals = contract.view_token_decimals(&pair_data.buy_token);

        assert_eq!(sell_token_decimals, 24);
        assert_eq!(buy_token_decimals, 18)
    }

    #[ignore]
    #[test]
    fn test_view_pending_limit_orders_by_user() {
        let contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );

        println!("{:#?}", contract.view_pending_limit_orders_by_user(alice()));
    }

    #[ignore]
    #[test]
    fn test_view_pending_limit_orders_by_user_by_pair() {
        let contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );

        println!(
            "{:#?}",
            contract.view_pending_limit_orders_by_user_by_pair(alice(), bob(), alice())
        );
    }

    #[test]
    fn test_take_profit_order_view() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = Contract::new_with_config(
            "owner_id.testnet".parse().unwrap(),
            "oracle_account_id.testnet".parse().unwrap(),
        );

        let pair_data = TradePair {
            sell_ticker_id: "usdt".to_string(),
            sell_token: "usdt.qa.v1.nearlend.testnet".parse().unwrap(),
            sell_token_decimals: 24,
            sell_token_market: "usdt_market.qa.v1.nearlend.testnet".parse().unwrap(),
            buy_ticker_id: "wnear".to_string(),
            buy_token: "wnear.qa.v1.nearlend.testnet".parse().unwrap(),
            buy_token_decimals: 18,
            pool_id: "usdt.qa.v1.nearlend.testnet|wnear.qa.v1.nearlend.testnet|2000".to_string(),
            max_leverage: U128(25 * 10_u128.pow(23)),
            swap_fee: U128(3 * 10_u128.pow(20)),
        };
        contract.add_pair(pair_data.clone());

        let order_string = "{\"status\":\"Pending\",\"order_type\":\"Buy\",\"amount\":1000000000,\"sell_token\":\"usdt.qa.v1.nearlend.testnet\",\"buy_token\":\"wnear.qa.v1.nearlend.testnet\",\"leverage\":\"1\",\"sell_token_price\":{\"ticker_id\":\"USDT\",\"value\":\"1.01\"},\"buy_token_price\":{\"ticker_id\":\"WNEAR\",\"value\":\"3.05\"},\"block\":103930910,\"lpt_id\":\"usdt.qa.v1.nearlend.testnet|wnear.qa.v1.nearlend.testnet|2000#540\"}".to_string();
        contract.add_order_from_string(alice(), order_string);

        let new_price = U128(5);
        contract.add_take_profit_order(U128(1), new_price);

        let tpo = contract.take_profit_orders.get(&1).unwrap();
        assert_eq!(tpo.status, OrderStatus::Pending);
        assert_eq!(tpo.buy_token_price.value, BigDecimal::from(new_price));

        let tpo_view = contract.take_profit_order_view(U128(1));
        assert_eq!(tpo_view.take_profit_price, new_price);
    }
}
