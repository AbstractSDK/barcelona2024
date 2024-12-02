use abstract_app::objects::namespace::Namespace;
use abstract_client::AbstractClient;
use abstract_client::RemoteAccount;
use cosmwasm_std::{coin, CosmosMsg};
use cw_orch::daemon::RUNTIME;
use cw_orch::daemon::{
    networks::{OSMO_5, PION_1},
    Daemon,
};
use cw_orch::prelude::*;
use cw_orch_interchain::daemon::{ChannelCreationValidator, DaemonInterchain};
use cw_orch_interchain::prelude::*;
use neutron_std::types::cosmos::base::v1beta1::Coin;
use neutron_std::types::ibc::applications::transfer::v1::MsgTransfer;
use neutron_std::types::osmosis::tokenfactory::v1beta1::{MsgCreateDenom, MsgMint};
use osmosis_std::types::osmosis::gamm::poolmodels::balancer::v1beta1::MsgCreateBalancerPool;
use osmosis_std::types::osmosis::gamm::v1beta1::PoolAsset;
use osmosis_std::types::osmosis::gamm::v1beta1::PoolParams;
use prost::Message as _;
use prost_13::Message as _;
use queriers::Ibc;

pub const SUB_DENOM_1: &str = "bitcoin";
pub const SUB_DENOM_2: &str = "ethereum";
pub const NAMESPACE: &str = "paymentapp-ibc"; // Only letters and hyphens
pub const INITIAL_MINT_AMOUNT: u128 = 100_000_000_000u128;
pub const INITIAL_TRANSFER_AMOUNT: u128 = 100_000_000u128;
pub const POOL_LIQUIDITY: u128 = 10_000_000u128;

pub const NEUTRON_ORIGIN_TRANSFER_CHANNEL: &str = "channel-1141";
pub const OSMOSIS_ORIGIN_TRANSFER_CHANNEL: &str = "channel-8851";

fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let interchain = DaemonInterchain::new(vec![PION_1, OSMO_5], &ChannelCreationValidator)?;

    let src_chain = interchain.get_chain("pion-1")?;
    let dst_chain = interchain.get_chain("osmo-test-5")?;

    // create_local_denoms(&src_chain)?;
    let interchain_account = create_interchain_account(&src_chain, &dst_chain, &interchain)?;

    // transfer_denoms(
    //     &src_chain,
    //     &interchain_account,
    //     &dst_chain,
    //     interchain.clone(),
    // )?;

    let pool_id = create_remote_pool(&src_chain, &dst_chain, &interchain_account)?;

    // install_payment_app(&src_chain, &dst_chain)?;

    Ok(())
}

fn new_denom(chain: &Daemon, sub_denom: &str) -> String {
    format!("factory/{}/{}", chain.sender_addr(), sub_denom)
}

fn ibc_trace(src_chain: &Daemon, dst_chain: &Daemon, sub_denom: &str) -> anyhow::Result<String> {
    let ibc: Ibc = dst_chain.querier();
    let hash = RUNTIME.block_on(ibc._denom_hash(format!(
        "transfer/{}/{}",
        OSMOSIS_ORIGIN_TRANSFER_CHANNEL,
        new_denom(src_chain, sub_denom)
    )))?;
    Ok(format!("ibc/{}", hash))
}

fn create_sub_denom(chain: &Daemon, subdenom: &str) -> anyhow::Result<()> {
    // Create some liquidity
    let denom_creation = MsgCreateDenom {
        sender: chain.sender_addr().to_string(),
        subdenom: subdenom.to_string(),
    };
    // Mint to myself
    let denom_mint = MsgMint {
        sender: chain.sender_addr().to_string(),
        amount: Some(neutron_std::types::cosmos::base::v1beta1::Coin {
            denom: new_denom(chain, subdenom),
            amount: INITIAL_MINT_AMOUNT.to_string(),
        }),
        mint_to_address: chain.sender_addr().to_string(),
    };

    chain.commit_any(
        vec![
            prost_types::Any {
                type_url: MsgCreateDenom::TYPE_URL.to_string(),
                value: denom_creation.encode_to_vec(),
            },
            prost_types::Any {
                type_url: MsgMint::TYPE_URL.to_string(),
                value: denom_mint.encode_to_vec(),
            },
        ],
        None,
    )?;
    Ok(())
}

fn create_local_denoms(chain: &Daemon) -> anyhow::Result<()> {
    create_sub_denom(chain, SUB_DENOM_1)?;
    create_sub_denom(chain, SUB_DENOM_2)?;
    Ok(())
}

fn create_interchain_account(
    src_chain: &Daemon,
    dst_chain: &Daemon,
    interchain: &DaemonInterchain,
) -> anyhow::Result<RemoteAccount<Daemon, DaemonInterchain>> {
    let src_client = AbstractClient::new(src_chain.clone())?;
    let dst_client = AbstractClient::new(dst_chain.clone())?;
    let namespace = Namespace::new(NAMESPACE)?;
    let src_account =
        src_client.fetch_or_build_account(namespace.clone(), |b| b.namespace(namespace.clone()))?;

    // Enabling IBC on the account
    if !src_account.ibc_status()? {
        src_account.set_ibc_status(true)?;
    }
    // Getting the existing remote account or creating it
    let remote_account = src_account
        .remote_account(interchain.clone(), dst_chain.clone())
        .or_else(|_| {
            src_account
                .remote_account_builder(interchain.clone(), &dst_client)
                .build()
        })?;

    Ok(remote_account)
}

fn transfer_subdenom_msg(
    src_chain: &Daemon,
    remote_account: &RemoteAccount<Daemon, DaemonInterchain>,
    dst_chain: &Daemon,
    sub_denom: &str,
) -> anyhow::Result<prost_types::Any> {
    let transfer_msg = MsgTransfer {
        source_port: "transfer".to_string(),
        source_channel: NEUTRON_ORIGIN_TRANSFER_CHANNEL.to_string(),
        token: Some(Coin {
            denom: new_denom(src_chain, sub_denom),
            amount: INITIAL_TRANSFER_AMOUNT.to_string(),
        }),
        sender: src_chain.sender_addr().to_string(),
        receiver: remote_account.address()?.to_string(),
        timeout_height: None,
        timeout_timestamp: src_chain.block_info()?.time.plus_hours(1).nanos(),
        memo: "".to_string(),
    };

    Ok(prost_types::Any {
        type_url: MsgTransfer::TYPE_URL.to_string(),
        value: transfer_msg.encode_to_vec(),
    })
}

fn transfer_denoms(
    src_chain: &Daemon,
    remote_account: &RemoteAccount<Daemon, DaemonInterchain>,
    dst_chain: &Daemon,
    interchain: DaemonInterchain,
) -> anyhow::Result<()> {
    let transfer_denom_1 =
        transfer_subdenom_msg(src_chain, remote_account, dst_chain, SUB_DENOM_1)?;
    let transfer_denom_2 =
        transfer_subdenom_msg(src_chain, remote_account, dst_chain, SUB_DENOM_2)?;
    let transfer_response = src_chain.commit_any(vec![transfer_denom_1, transfer_denom_2], None)?;
    interchain.await_and_check_packets("pion-1", transfer_response)?;
    Ok(())
}

fn create_remote_pool(
    src_chain: &Daemon,
    dst_chain: &Daemon,
    remote_account: &RemoteAccount<Daemon, DaemonInterchain>,
) -> anyhow::Result<()> {
    use abstract_std::account;
    todo!(
        "This needs some osmo to function correctly, it costs 1_000_000 uosmo to create a pool !"
    );
    let create_pool_msg = MsgCreateBalancerPool {
        sender: remote_account.address()?.to_string(),
        pool_params: Some(PoolParams {
            swap_fee: "10000000000000000".to_string(),
            exit_fee: "0".to_string(),
            smooth_weight_change_params: None,
        }),
        pool_assets: [
            coin(
                POOL_LIQUIDITY,
                ibc_trace(src_chain, dst_chain, SUB_DENOM_1)?,
            ),
            coin(
                POOL_LIQUIDITY,
                ibc_trace(src_chain, dst_chain, SUB_DENOM_2)?,
            ),
        ]
        .iter()
        .map(|c| PoolAsset {
            token: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
                denom: c.denom.to_owned(),
                amount: format!("{}", c.amount),
            }),
            weight: "1000000".to_string(),
        })
        .collect(),
        future_pool_governor: "".to_string(),
    };
    let response = remote_account.execute_on_account(vec![account::ExecuteMsg::Execute {
        msgs: vec![CosmosMsg::Stargate {
            type_url: MsgCreateBalancerPool::TYPE_URL.to_string(),
            value: create_pool_msg.encode_to_vec().into(),
        }],
    }])?;

    let pool_id = todo!();
    //response.packets[0].ack_tx.tx_id.get_events("pool_created")[0].get_attributes("pool_id")[0]
        .value
        .parse()?;
    Ok(pool_id)
}
