// Code generated by the dharitri-sc build system. DO NOT EDIT.

////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

// Init:                                 1
// Upgrade:                              1
// Endpoints:                           11
// Async Callback (empty):               1
// Total number of exported functions:  14

#![no_std]

dharitri_sc_wasm_adapter::allocator!();
dharitri_sc_wasm_adapter::panic_handler!();

dharitri_sc_wasm_adapter::endpoints! {
    ping_pong_moa
    (
        init => init
        upgrade => upgrade
        ping => ping
        pong => pong
        pongAll => pong_all
        getUserAddresses => get_user_addresses
        getContractState => get_contract_state
        getPingAmount => ping_amount
        getDeadline => deadline
        getActivationTimestamp => activation_timestamp
        getMaxFunds => max_funds
        getUserStatus => user_status
        pongAllLastUser => pong_all_last_user
    )
}

dharitri_sc_wasm_adapter::async_callback_empty! {}