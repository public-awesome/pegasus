use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, TokenMsg};
use crate::state::{SudoParams, SUDO_PARAMS, Token, Offer, next_offer_id, offers};
use crate::query::query_offers_by_sender;
// use crate::query::{query_offers_by_sender};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Timestamp, Deps, WasmMsg, to_binary, SubMsg};
use cw2::set_contract_version;
use cw721::{OwnerOfResponse, Cw721ExecuteMsg};
use cw721_base::helpers::Cw721Contract;
use sg_std::Response;

// Version info for migration info
const CONTRACT_NAME: &str = "crates.iosg-p2p-nft-trade";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let params = SudoParams { 
        escrow_deposit_amount: msg.escrow_deposit_amount, 
        offer_expiry: msg.offer_expiry, 
        maintainer: deps.api.addr_validate(&msg.maintainer)?, 
        removal_reward_bps: msg.removal_reward_bps 
    };
    SUDO_PARAMS.save(deps.storage, &params)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;

        match msg {
        ExecuteMsg::CreateOffer { 
            offered_nfts, 
            wanted_nfts, 
            peer, 
            expires_at 
        } => execute_create_offer(deps, env, info, 
            offered_nfts.into_iter().map(|nft: TokenMsg| Token { collection: api.addr_validate(&nft.collection).unwrap(), token_id: nft.token_id }).collect(),
            wanted_nfts.into_iter().map(|nft: TokenMsg| Token { collection: api.addr_validate(&nft.collection).unwrap(), token_id: nft.token_id }).collect(),
            api.addr_validate(&peer)?, 
            expires_at),
            
        ExecuteMsg::RemoveOffer { id } => execute_remove_offer(deps, info, id),
        ExecuteMsg::AcceptOffer { id } => execute_accept_offer(deps, env, info, id),
        ExecuteMsg::RejectOffer { id } => execute_reject_offer(deps, info, id),
        ExecuteMsg::RemoveStaleOffer { id } => execute_remove_stale_offer(deps, env, info, id),
    }
}


fn execute_create_offer(deps: DepsMut, env: Env, info: MessageInfo, offered_tokens: Vec<Token>, wanted_tokens: Vec<Token>, peer: Addr, expires_at: Timestamp ) -> Result<Response, ContractError> {
    // check if the sender is the owner of the tokens
    for token in offered_tokens.clone() {
        // TODO: [OPTIMISATION] See if we can levarage the OwnerOfResponse.Approvals for checking if the contract has been approved
        let _ = only_owner(deps.as_ref(), &info, &token.collection, token.token_id)?;
        
        // check if the contract is approved to send transfer the tokens
        Cw721Contract(token.collection.clone()).approval(
            &deps.querier,
            token.token_id.to_string(),
            env.contract.address.to_string(),
            None,
        )?;

        // check if the tokens arent already offered in another trade
        let current_sender_offers = query_offers_by_sender(deps.as_ref(), info.sender.clone())?.offers;
        for offer in current_sender_offers {
            if offer.offered_nfts.contains(&token) {
                return Err(ContractError::TokenAlreadyOffered { collection: token.collection.into_string(), token_id: token.token_id, offer_id: offer.id })
            }
        }
    }
    
    // check if the peer is the owner of the requested tokens
    for token in wanted_tokens.clone() {
        if peer.to_string() != Cw721Contract(token.collection.clone()).owner_of(&deps.querier, token.token_id.to_string(), false)?.owner {
            return Err(ContractError::UnauthorizedPeer { collection: token.collection.to_string(), token_id: token.token_id, peer: peer.into_string() })
        }
    }
    
    // check if the expiry date is valid
    SUDO_PARAMS.load(deps.storage)?.offer_expiry.is_valid(&env.block, expires_at)?;
    
    // create and save offer 
    let offer = Offer { 
        id: next_offer_id(deps.storage)?, 
        offered_nfts:offered_tokens, 
        wanted_nfts: wanted_tokens, 
        sender: info.sender, 
        peer: peer, 
        expires_at: expires_at, 
    };
    offers().save(deps.storage, &[offer.id], &offer)?;
    
    Ok(Response::new()
        .add_attribute("action", "create_offer")
        .add_attribute("offer_id", offer.id.to_string())
    )
}

fn execute_remove_offer(deps:DepsMut,info: MessageInfo, id:u8) -> Result<Response, ContractError> {
    // check if the sender of this msg is the sender of the offer
    let offer = offers().load(deps.as_ref().storage, &[id])?;
    if offer.sender != info.sender {
        return Err(ContractError::UnauthorizedOperator {  });
    }

    offers().remove(deps.storage, &[offer.id])?;

    Ok(Response::new()
        .add_attribute("action", "revoke_offer")
        .add_attribute("offer_id", offer.id.to_string())
    )
}

fn execute_accept_offer(deps:DepsMut, env: Env, info: MessageInfo, id:u8) -> Result<Response, ContractError> {
    let offer = offers().load(deps.storage, &[id])?;

    let params = SUDO_PARAMS.load(deps.storage)?;


    // check if the sender is the peer of the offer
    if offer.peer != info.sender {
        return Err(ContractError::UnauthorizedOperator {  })
    }

    // check if the offer is not yet expired
    params.offer_expiry.is_valid(&env.block, offer.expires_at)?;

    // check if the sender owns the requested nfts
    for token in offer.wanted_nfts.clone() {
        only_owner(deps.as_ref(), &info, &token.collection, token.token_id)?;

        // check if the contract is approved to send transfer the tokens
        Cw721Contract(token.collection.clone()).approval(
            &deps.querier,
            token.token_id.to_string(),
            env.contract.address.to_string(),
            None,
        )?;
    }

    // check if the offeror owns the offered nfts
    for token in offer.offered_nfts.clone() {
        if offer.sender != Cw721Contract(token.collection.clone()).owner_of(&deps.querier, token.token_id.to_string(), false)?.owner {
            return Err(ContractError::UnauthorizedPeer { collection: token.collection.into_string(), token_id: token.token_id, peer: offer.sender.to_string() })
        }

        // check if the contract is approved to send transfer the tokens
        Cw721Contract(token.collection.clone()).approval(
            &deps.querier,
            token.token_id.to_string(),
            env.contract.address.to_string(),
            None,
        )?;
    }
    let mut res = Response::new();
    
    // remove the offer
    offers().remove(deps.storage, &[offer.id])?;

    // transfer nfts
    transfer_nfts(offer.peer.to_string(), offer.offered_nfts.clone(), &mut res)?;
    transfer_nfts(offer.sender.to_string(), offer.wanted_nfts.clone(), &mut res)?;


    Ok(res
        .add_attribute("action", "accept_offer")
    )
}

fn transfer_nfts(recipient: String, nfts: Vec<Token>, res: &mut cosmwasm_std::Response<sg_std::StargazeMsgWrapper>) -> Result<(), ContractError> {
    Ok(for token in nfts {
        let cw721_transfer_msg = Cw721ExecuteMsg::TransferNft { recipient: recipient.clone(), token_id: token.token_id.to_string() };
        let exec_cw721_transfer_msg = WasmMsg::Execute { contract_addr: token.collection.to_string(), msg: to_binary(&cw721_transfer_msg)?, funds: vec![] };
    
        res.messages
            .push(SubMsg::new(exec_cw721_transfer_msg));
    })
}

fn execute_reject_offer(deps:DepsMut, info: MessageInfo, id:u8) -> Result<Response, ContractError> {
    // check if the sender of this msg is the peer of the offer
    let offer = offers().load(deps.as_ref().storage, &[id])?;
    if offer.peer != info.sender {
        return Err(ContractError::UnauthorizedOperator {  });
    }

    offers().remove(deps.storage, &[offer.id])?;

    Ok(Response::new()
        .add_attribute("action", "reject_offer")
        .add_attribute("offer_id", offer.id.to_string())
    )
}

fn execute_remove_stale_offer(deps:DepsMut, env: Env, info: MessageInfo, id:u8) -> Result<Response, ContractError> {
    let offer = offers().load(deps.storage, &[id])?;

    let params = SUDO_PARAMS.load(deps.storage)?;

    params.offer_expiry.is_valid(&env.block, offer.expires_at)?;

    if info.sender != params.maintainer {
        return Err(ContractError::UnauthorizedOperator {  })
    }

    offers().remove(deps.storage, &[id])?;

    Ok(Response::new()
        .add_attribute("action", "remove_stale_offer")
        .add_attribute("offer_id", offer.id.to_string())
    )
}


// ---------------------------------------------------------------------------------
// helper functions
// ---------------------------------------------------------------------------------


/// Checks to enfore only NFT owner can call
fn only_owner(
    deps: Deps,
    info: &MessageInfo,
    collection: &Addr,
    token_id: u32,
) -> Result<OwnerOfResponse, ContractError> {
    let res =
        Cw721Contract(collection.clone()).owner_of(&deps.querier, token_id.to_string(), false)?;
    if res.owner != info.sender {
        return Err(ContractError::UnauthorizedOwner {});
    }

    Ok(res)
}

// fn finalize_trade(deps: Deps, offered: Vec<Token>) {}