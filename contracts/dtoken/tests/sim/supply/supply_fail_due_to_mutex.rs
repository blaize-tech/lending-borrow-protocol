// use near_sdk::{json_types::U128, serde_json::json, Balance};
// use near_sdk_sim::{call, init_simulator, view, ContractAccount, UserAccount};

// use controller::ActionType::Supply;
// use general::Price;

// use crate::utils::{
//     add_market, initialize_controller, initialize_two_dtokens, initialize_two_utokens, new_user,
//     view_balance, mint_tokens, set_price,
// };

// const WNEAR_BALANCE: Balance = 50;
// const WETH_BALANCE: Balance = 100;
// const SUPPLY_WETH_AMOUNT: Balance = 50;
// const START_PRICE: Balance = 10000;

// fn supply_fail_due_to_mutex_fixture() -> (
//     ContractAccount<dtoken::ContractContract>,
//     ContractAccount<dtoken::ContractContract>,
//     ContractAccount<controller::ContractContract>,
//     ContractAccount<test_utoken::ContractContract>,
//     ContractAccount<test_utoken::ContractContract>,
//     UserAccount,
// ) {
//     let root = init_simulator(None);

//     let user = new_user(&root, "user".parse().unwrap());
//     let (weth, wnear) = initialize_two_utokens(&root);
//     let controller = initialize_controller(&root);
//     let (dweth, dwnear) = initialize_two_dtokens(
//         &root,
//         weth.account_id(),
//         wnear.account_id(),
//         controller.account_id(),
//     );

//     mint_tokens(&wnear, dwnear.account_id(), U128(100));
//     mint_tokens(&wnear, user.account_id(), U128(WETH_BALANCE));
//     mint_tokens(&weth, dweth.account_id(), U128(100));
//     mint_tokens(&weth, user.account_id(), U128(WNEAR_BALANCE));

//     add_market(
//         &controller,
//         weth.account_id(),
//         dweth.account_id(),
//         "weth".to_string(),
//     );

//     add_market(
//         &controller,
//         wnear.account_id(),
//         dwnear.account_id(),
//         "weth".to_string(),
//     );

//     set_price(
//         &controller,
//         dwnear.account_id(),
//         &Price {
//             ticker_id: "wnear".to_string(),
//             value: U128(START_PRICE),
//             volatility: U128(100),
//             fraction_digits: 4,
//         },
//     );

//     set_price(
//         &controller,
//         dweth.account_id(),
//         &Price {
//             ticker_id: "weth".to_string(),
//             value: U128(START_PRICE),
//             volatility: U128(100),
//             fraction_digits: 4,
//         },
//     );

//     (dweth, dwnear, controller, weth, wnear, user)
// }

// #[test]
// fn scenario_supply_fail_due_to_mutex() {
//     let (dweth, dwnear, controller, weth, wnear, user) = supply_fail_due_to_mutex_fixture();

//     let action = "\"Supply\"".to_string();

//     let tx = user
//         .create_transaction(weth.account_id())
//         .function_call(
//             "ft_transfer_call".to_string(),
//             json!({
//                 "receiver_id": dweth.account_id(),
//                 "amount": U128(SUPPLY_WETH_AMOUNT),
//                 "memo": Some("SUPPLY".to_string()),
//                 "msg": action.clone()
//             })
//             .to_string()
//             .into_bytes(),
//             170000000000000,
//             1,
//         )
//         // .function_call(
//         //     "ft_transfer_call".to_string(),
//         //     json!({
//         //         "receiver_id": dweth.account_id(),
//         //         "amount": U128(SUPPLY_WETH_AMOUNT),
//         //         "memo": Some("SUPPLY".to_string()),
//         //         "msg": action.clone()
//         //     })
//         //     .to_string()
//         //     .into_bytes(),
//         //     130000000000000,
//         //     1,
//         // )
//         .submit();
//     println!("{:?}", tx);
//     tx.assert_success();

//     let user_balance: U128 = view!(weth.ft_balance_of(user.account_id())).unwrap_json();
//     assert_eq!(
//         user_balance,
//         U128(SUPPLY_WETH_AMOUNT),
//         "User balance should be 0"
//     );

//     let user_balance: Balance =
//         view_balance(&controller, Supply, user.account_id(), dweth.account_id());
//     assert_eq!(
//         user_balance, SUPPLY_WETH_AMOUNT,
//         "Balance on controller should be {}",
//         SUPPLY_WETH_AMOUNT
//     );
// }
