use abstract_app::sdk::cw_helpers::Clearable;
use abstract_app::std::objects::{AnsAsset, AssetEntry, DexName};
use cosmwasm_schema::QueryResponses;
use cosmwasm_std::Addr;

use crate::contract::PaymentApp;

// TODO 1
// Understand where you create your messages, and what you can change in your business logic

// This is used for type safety
// The second part is used to indicate the messages are used as the apps messages
// This is equivalent to
// pub type InstantiateMsg = <PaymentApp as abstract_sdk::base::InstantiateEndpoint>::InstantiateMsg;
// pub type ExecuteMsg = <PaymentApp as abstract_sdk::base::ExecuteEndpoint>::ExecuteMsg;
// pub type QueryMsg = <PaymentApp as abstract_sdk::base::QueryEndpoint>::QueryMsg;
// pub type MigrateMsg = <PaymentApp as abstract_sdk::base::MigrateEndpoint>::MigrateMsg;

// impl app::AppExecuteMsg for AppExecuteMsg {}
// impl app::AppQueryMsg for AppQueryMsg {}
abstract_app::app_msg_types!(PaymentApp, AppExecuteMsg, AppQueryMsg);

/// PaymentApp instantiate message
#[cosmwasm_schema::cw_serde]
pub struct AppInstantiateMsg {
    pub desired_asset: Option<AssetEntry>,
    pub denom_asset: String,
    pub exchanges: Vec<DexName>,
}

/// PaymentApp execute messages
#[cosmwasm_schema::cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum AppExecuteMsg {
    // TODO 3 : This endpoint should receive some funds, so cw-orch should know that
    // https://orchestrator.abstract.money/contracts/entry-points.html?highlight=payable#payable-attribute
    Tip {},
    UpdateConfig {
        desired_asset: Option<Clearable<AssetEntry>>,
        denom_asset: Option<String>,
        exchanges: Option<Vec<DexName>>,
    },
}

#[cosmwasm_schema::cw_serde]
#[derive(cw_orch::ExecuteFns)]
/// Custom execute handler
pub enum CustomExecuteMsg {
    /// A configuration message, defined by the base.
    Base(abstract_app::std::app::BaseExecuteMsg),
    /// An app request defined by a base consumer.
    Module(AppExecuteMsg),
    // Payment App doesn't use IBC, so those skipped here
    /// Custom msg type
    Receive(cw20::Cw20ReceiveMsg),
}

// Enable cw_orch api
impl From<AppExecuteMsg> for CustomExecuteMsg {
    fn from(value: AppExecuteMsg) -> Self {
        Self::Module(value)
    }
}

#[cosmwasm_schema::cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum AppQueryMsg {
    /// Returns [`ConfigResponse`]
    #[returns(ConfigResponse)]
    Config {},
    /// Returns [`TipperResponse`]
    #[returns(TipperResponse)]
    Tipper {
        address: String,
        start_after: Option<AssetEntry>,
        limit: Option<u32>,
        at_height: Option<u64>,
    },
    /// Returns [`TipCountResponse`]
    #[returns(TipCountResponse)]
    TipCount {},
    /// Returns [`TippersCountResponse`]
    #[returns(TippersCountResponse)]
    ListTippersCount {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cosmwasm_schema::cw_serde]
pub struct AppMigrateMsg {}

#[cosmwasm_schema::cw_serde]
pub struct Cw20TipMsg {}

#[cosmwasm_schema::cw_serde]
pub struct ConfigResponse {
    pub desired_asset: Option<AssetEntry>,
    pub denom_asset: String,
    pub exchanges: Vec<DexName>,
}

#[cosmwasm_schema::cw_serde]
pub struct TipperResponse {
    pub address: Addr,
    pub tip_count: u32,
    pub total_amounts: Vec<AnsAsset>,
}

#[cosmwasm_schema::cw_serde]
pub struct TipperCountResponse {
    pub address: Addr,
    pub count: u32,
}

#[cosmwasm_schema::cw_serde]
pub struct TippersCountResponse {
    pub tippers: Vec<TipperCountResponse>,
}

#[cosmwasm_schema::cw_serde]
pub struct TipCountResponse {
    pub count: u32,
}
