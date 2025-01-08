use dharitri_chain_core::types::ReturnCode;

use crate::{
    chain_core::builtin_func_names::DCDT_NFT_ADD_URI_FUNC_NAME,
    tx_execution::BlockchainVMRef,
    tx_mock::{BlockchainUpdate, TxCache, TxInput, TxLog, TxResult},
    types::{top_decode_u64, top_encode_u64},
};

use super::super::builtin_func_trait::BuiltinFunction;

pub struct DCDTNftAddUri;

impl BuiltinFunction for DCDTNftAddUri {
    fn name(&self) -> &str {
        DCDT_NFT_ADD_URI_FUNC_NAME
    }

    fn execute<F>(
        &self,
        tx_input: TxInput,
        tx_cache: TxCache,
        _vm: &BlockchainVMRef,
        _f: F,
    ) -> (TxResult, BlockchainUpdate)
    where
        F: FnOnce(),
    {
        if tx_input.args.len() < 3 {
            let err_result = TxResult::from_vm_error("DCDTNFTAddURI expects at least 3 arguments");
            return (err_result, BlockchainUpdate::empty());
        }

        let token_identifier = tx_input.args[0].clone();
        let nonce = top_decode_u64(tx_input.args[1].as_slice());
        let mut new_uris = tx_input.args[2..].to_vec();

        tx_cache.with_account_mut(&tx_input.from, |account| {
            account
                .dcdt
                .add_uris(token_identifier.as_slice(), nonce, new_uris.clone());
        });

        let mut topics = vec![
            token_identifier.to_vec(),
            top_encode_u64(nonce),
            Vec::new(), // value = 0
        ];
        topics.append(&mut new_uris);
        let dcdt_nft_create_log = TxLog {
            address: tx_input.from,
            endpoint: DCDT_NFT_ADD_URI_FUNC_NAME.into(),
            topics,
            data: vec![],
        };

        let tx_result = TxResult {
            result_status: ReturnCode::Success,
            result_logs: vec![dcdt_nft_create_log],
            ..Default::default()
        };

        (tx_result, tx_cache.into_blockchain_updates())
    }
}