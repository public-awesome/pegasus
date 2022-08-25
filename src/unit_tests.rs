#[cfg(test)]

use crate::error::ContractError;

use crate::execute::execute;
use crate::msg::ExecuteMsg;
use crate::state::offers;
use crate::{msg::InstantiateMsg, ExpiryRange, execute::instantiate, state::{Offer, Token}};

use cosmwasm_std::{testing::*, DepsMut, Addr, coins, Timestamp, StdError, Empty};
use cw721_base::{Cw721Contract, Extension, InstantiateMsg as Cw721InstantiateMsg, ExecuteMsg as Cw721ExecuteMsg, MintMsg};



const CREATOR: &str = "creator";
const COLLECTION_A: &str = "collection-a";
const COLLECTION_B: &str = "collection-b";
const TOKEN1_ID: u32 = 123;
const TOKEN2_ID: u32 = 234;
const TOKEN3_ID: u32 = 345;
const TOKEN4_ID: u32 = 456;

const SENDER: &str = "sender";
// const SENDER2: &str = "sender";
const PEER: &str = "peer";
const MAX_EXPIRY: u64 = 60;
const MIN_EXPIRY: u64 = 60;

//---------------------------------------------------------
// Unit tests without Cw721Queries
//---------------------------------------------------------
#[test]

fn proper_initialization() {
    let mut deps = mock_dependencies();

    instantiate_trade_contract(deps.as_mut());
}

// test remove/reject
#[test]
fn remove_offer() {
    let mut deps = mock_dependencies();

    let collection = Addr::unchecked(COLLECTION_A);
    let sender = Addr::unchecked(SENDER);
    let peer = Addr::unchecked(PEER);
    
    instantiate_trade_contract(deps.as_mut());
    
    let mock_sender_info = mock_info(SENDER, &[]);
    let mock_peer = mock_info(PEER, &[]);

    let offered_nfts = vec![Token{collection: collection.clone(), token_id: TOKEN1_ID}];
    let wanted_nfts = vec![Token{collection, token_id: TOKEN2_ID}];

    save_new_offer(deps.as_mut(), SENDER, PEER, offered_nfts, wanted_nfts);

    let exec_msg = ExecuteMsg::RemoveOffer { id: 0 };
    
    // The peer of the offer should not be able to remove it
    let res = execute(deps.as_mut(), mock_env(), mock_peer, exec_msg.clone());
    assert!(res.is_err(), "Peer is able to remove offer.");
    assert_eq!(res.unwrap_err(), ContractError::UnauthorizedSender { });
    

    // The sender of the offer should be able to remove it
    let res = execute(deps.as_mut(), mock_env(), mock_sender_info.clone(), exec_msg);
    assert!(res.is_ok(), "Sender of the Offer cant remove");
    
    // test for non existing offer
    let remove_nonexisting_msg = ExecuteMsg::RemoveOffer { id: 1 };
    
    let err = execute(deps.as_mut(), mock_env(), mock_sender_info, remove_nonexisting_msg).unwrap_err();
    assert_eq!(err, ContractError::Std(StdError::NotFound { kind: "sg_p2p_nft_trade::state::Offer".to_string() }), "Error should be of type notFound.")
}

#[test]
fn reject_offer() {
    let mut deps = mock_dependencies();

    let collection = Addr::unchecked(COLLECTION_B);
    let sender = Addr::unchecked(SENDER);
    let peer = Addr::unchecked(PEER);
    
    instantiate_trade_contract(deps.as_mut());
    
    let mock_sender_info = mock_info(SENDER, &[]);
    let mock_peer = mock_info(PEER, &[]);

    let offered_nfts = vec![Token{collection: collection.clone(), token_id: TOKEN1_ID}];
    let wanted_nfts = vec![Token{collection, token_id: TOKEN2_ID}];

    save_new_offer(deps.as_mut(), SENDER, PEER, offered_nfts, wanted_nfts);

    let exec_msg = ExecuteMsg::RejectOffer { id: 0 };
    
    
    // The sender of the offer should not be able to reject his own offer
    let res = execute(deps.as_mut(), mock_env(), mock_sender_info.clone(), exec_msg.clone());
    assert!(res.is_err(), "Sender is able to reject offer.");
    
    // The peer of the offer should be able to reject the offer
    let res = execute(deps.as_mut(), mock_env(), mock_peer, exec_msg.clone());
    assert!(res.is_ok(), "Peer of the Offer cant reject");

    // test for non existing offer
    let remove_nonexisting_msg = ExecuteMsg::RejectOffer { id: 1 };
    
    let err = execute(deps.as_mut(), mock_env(), mock_sender_info, remove_nonexisting_msg).unwrap_err();
    assert_eq!(err, ContractError::Std(StdError::NotFound { kind: "sg_p2p_nft_trade::state::Offer".to_string() }), "Error should be of type notFound.") 
}


//---------------------------------------------------------
// test helpers 
//---------------------------------------------------------
fn token_uri(collection: &str, token_id: &str) ->String {
    "https://www.maurits-bos.me/nfts/".to_string() + collection + token_id
}

// helper that injects a offer into the database
fn save_new_offer( deps: DepsMut, sender: &str, peer: &str, offered_nfts: Vec<Token>, wanted_nfts: Vec<Token>) {
let collection = Addr::unchecked(COLLECTION_A);
    let sender = Addr::unchecked(sender);
    let peer = Addr::unchecked(peer);
    
    let offer = Offer {
        id: 0,
        offered_nfts: offered_nfts,
        wanted_nfts: wanted_nfts,
        sender: sender,
        peer: peer,
        expires_at: Timestamp::from_seconds(1_000_000),
    };
    let res = offers().save( deps.storage, &[offer.id], &offer);
    assert!(res.is_ok(), "Failed to save offer to storage");
}

// setup contract helper 
fn instantiate_trade_contract(deps: DepsMut) {
    let msg = InstantiateMsg {
        escrow_deposit_amount: cosmwasm_std::Uint128::new(0),
        offer_expiry: ExpiryRange { min: MIN_EXPIRY, max: MAX_EXPIRY },
        maintainer: CREATOR.to_owned(),
        removal_reward_bps: 0,
    };
    let info = mock_info(CREATOR, &[]);
    let res = instantiate(deps, mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}