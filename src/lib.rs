pub use crate::msg::{InstantiateMsg, QueryMsg};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Empty;
pub use cw721_base::{
    entry::query as _query,
    ContractError, Cw721Contract, InstantiateMsg as Cw721BaseInstantiateMsg,
    MinterResponse,
};

pub mod msg;
pub mod query;
pub mod state;

#[cw_serde]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}

#[cw_serde]
#[derive(Default)]
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<Vec<Trait>>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
}

pub type Extension = Option<Metadata>;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-non-transferable-with-metadata-onchain";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type Cw721NonTransferableContract<'a> = Cw721Contract<'a, Extension, Empty, Empty, Empty>;
pub type ExecuteMsg = cw721_base::ExecuteMsg<Extension, Empty>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;
    use crate::query::admin;
    use crate::state::{Config, CONFIG};
    use cosmwasm_std::{
        entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    };

    #[entry_point]
    pub fn instantiate(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let admin_addr: Option<Addr> = msg
            .admin
            .as_deref()
            .map(|s| deps.api.addr_validate(s))
            .transpose()?;

        let config = Config { admin: admin_addr };

        CONFIG.save(deps.storage, &config)?;

        let cw721_base_instantiate_msg = Cw721BaseInstantiateMsg {
            name: msg.name,
            symbol: msg.symbol,
            minter: msg.minter,
        };

        Cw721NonTransferableContract::default().instantiate(
            deps.branch(),
            env,
            info,
            cw721_base_instantiate_msg,
        )?;

        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        Ok(Response::default()
            .add_attribute("contract_name", CONTRACT_NAME)
            .add_attribute("contract_version", CONTRACT_VERSION))
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, cw721_base::ContractError> {
        let config = CONFIG.load(deps.storage)?;
        match config.admin {
            Some(admin) => {
                if admin == info.sender {
                    Cw721NonTransferableContract::default().execute(deps, env, info, msg)
                } else {
                    Err(ContractError::Ownership(
                        cw721_base::OwnershipError::NotOwner,
                    ))
                }
            }
            None => match msg {
                ExecuteMsg::Mint {
                    token_id,
                    owner,
                    token_uri,
                    extension,
                } => Cw721NonTransferableContract::default()
                    .mint(deps, info, token_id, owner, token_uri, extension),
                _ => Err(ContractError::Ownership(
                    cw721_base::OwnershipError::NotOwner,
                )),
            },
        }
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Admin {} => to_binary(&admin(deps)?),
            _ => _query(deps, env, msg.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cw721::Cw721Query;

    const MINTER: &str = "minter";    

    #[test]
    fn use_metadata_extension() {
        let mut deps = mock_dependencies();
        let contract = Cw721NonTransferableContract::default();

        let info = mock_info(MINTER, &[]);

        let cw721_base_instantiate_msg = Cw721BaseInstantiateMsg {
            name: "TEST TOKEN".to_string(),
            symbol: "TEST".to_string(),
            minter: MINTER.to_string(),
        };

        contract
            .instantiate(deps.as_mut(), mock_env(), info.clone(), cw721_base_instantiate_msg)
            .unwrap();
        
        let token_uri = None;
        let metadata_extension = Some(Metadata {
            description: Some("Description for metadata".into()),
            name: Some("TEST".to_string()),
            ..Metadata::default()
        });

        let token_id = "1";
        let mint_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: MINTER.to_string(),
            token_uri: token_uri.clone(),
            extension: metadata_extension.clone(),
        };
        contract
            .execute(deps.as_mut(), mock_env(), info, mint_msg)
            .unwrap();

        let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
        
        assert_eq!(res.token_uri, token_uri);
        assert_eq!(res.extension, metadata_extension);

    }
}