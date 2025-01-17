use cosmwasm_std::StdError;
use thiserror::Error;

use crate::helpers::ExpiryRangeError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Can't create an offer without nfts")]
    EmptyTokenVector {},

    #[error("UnauthorizedOwner")]
    UnauthorizedSender {},

    #[error("Cant offer to same address")]
    AlreadyOwned {},

    #[error("Contract is not authorized to spend token(collection: {collection:?}, token_id: {token_id:?}) ")]
    Unauthorized { collection: String, token_id: u32 },

    #[error("Token (collection: {collection:?}, id: {token_id:?}) is already offered in offer {offer_id:?}" )]
    TokenAlreadyOffered {
        collection: String,
        token_id: u32,
        offer_id: u8,
    },

    #[error(
        "address {peer:?} is not owner of Token (collection: {collection:?}, id: {token_id:?})"
    )]
    UnauthorizedPeer {
        collection: String,
        token_id: u32,
        peer: String,
    },

    #[error("UnauthorizedOperator")]
    UnauthorizedOperator {},
    #[error("Address {addr:?} cannot create more than {max_offers:?} offers")]
    MaxOffers { addr: String, max_offers: u64 },

    #[error("{0}")]
    ExpiryRange(#[from] ExpiryRangeError),
}
