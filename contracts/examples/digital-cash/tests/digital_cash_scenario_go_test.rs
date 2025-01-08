use dharitri_sc_scenario::*;

fn world() -> ScenarioWorld {
    ScenarioWorld::vm_go()
}

#[test]
fn claim_moa_go() {
    world().run("scenarios/claim-moa.scen.json");
}

#[test]
fn claim_dcdt_go() {
    world().run("scenarios/claim-dcdt.scen.json");
}

#[test]
fn claim_fees_go() {
    world().run("scenarios/claim-fees.scen.json");
}

#[test]
fn claim_multi_dcdt_go() {
    world().run("scenarios/claim-multi-dcdt.scen.json");
}

#[test]
fn forward_go() {
    world().run("scenarios/forward.scen.json");
}

#[test]
fn fund_moa_and_dcdt_go() {
    world().run("scenarios/fund-moa-and-dcdt.scen.json");
}

#[test]
fn set_accounts_go() {
    world().run("scenarios/set-accounts.scen.json");
}

#[test]
fn whitelist_blacklist_fee_token_go() {
    world().run("scenarios/whitelist-blacklist-fee-tokens.scen.json");
}

#[test]
fn pay_fee_and_fund_dcdt_go() {
    world().run("scenarios/pay-fee-and-fund-dcdt.scen.json");
}

#[test]
fn pay_fee_and_fund_moa_go() {
    world().run("scenarios/pay-fee-and-fund-moa.scen.json");
}

#[test]
fn withdraw_moa_go() {
    world().run("scenarios/withdraw-moa.scen.json");
}

#[test]
fn withdraw_dcdt_go() {
    world().run("scenarios/withdraw-dcdt.scen.json");
}

#[test]
fn withdraw_multi_dcdt_go() {
    world().run("scenarios/withdraw-multi-dcdt.scen.json");
}