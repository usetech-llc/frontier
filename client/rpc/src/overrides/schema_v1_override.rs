// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
// This file is part of Frontier.
//
// Copyright (c) 2017-2022 Parity Technologies (UK) Ltd.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{marker::PhantomData, sync::Arc};

use codec::Decode;
use ethereum_types::{H160, H256, U256};

use sc_client_api::backend::{Backend, StateBackend, StorageProvider};
use sp_api::{BlockId, ProvideRuntimeApi};
use sp_runtime::{
	traits::{BlakeTwo256, Block as BlockT},
	Permill,
};
use sp_storage::StorageKey;

use fp_rpc::{TransactionStatus, EthereumRuntimeRPCApi};

use super::{blake2_128_extend, storage_prefix_build, StorageOverride};

/// An override for runtimes that use Schema V1
pub struct SchemaV1Override<B: BlockT, C, BE> {
	client: Arc<C>,
	_marker: PhantomData<(B, BE)>,
}

impl<B: BlockT, C, BE> SchemaV1Override<B, C, BE> {
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: PhantomData,
		}
	}
}

impl<B, C, BE> SchemaV1Override<B, C, BE>
where
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C: StorageProvider<B, BE> + Send + Sync + 'static,
	BE: Backend<B> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
{
	fn query_storage<T: Decode>(&self, id: &BlockId<B>, key: &StorageKey) -> Option<T> {
		if let Ok(Some(data)) = self.client.storage(id, key) {
			if let Ok(result) = Decode::decode(&mut &data.0[..]) {
				return Some(result);
			}
		}
		None
	}
}

impl<Block, C, BE> StorageOverride<Block> for SchemaV1Override<Block, C, BE>
where
	C: ProvideRuntimeApi<Block>,
	C::Api: EthereumRuntimeRPCApi<Block>,
	C: StorageProvider<Block, BE> + Send + Sync + 'static,
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	Block: BlockT<Hash = H256> + Send + Sync + 'static,
	C: Send + Sync + 'static,
{
	/// For a given account address, returns pallet_evm::AccountCodes.
	fn account_code_at(&self, block: &BlockId<Block>, address: H160) -> Option<Vec<u8>> {
		let api = self.client.runtime_api();
		api.account_code_at(
			block,
			address
		).ok()
	}

	/// For a given account address and index, returns pallet_evm::AccountStorages.
	fn storage_at(&self, block: &BlockId<Block>, address: H160, index: U256) -> Option<H256> {
		let tmp: &mut [u8; 32] = &mut [0; 32];
		index.to_big_endian(tmp);

		let mut key: Vec<u8> = storage_prefix_build(b"EVM", b"AccountStorages");
		key.extend(blake2_128_extend(address.as_bytes()));
		key.extend(blake2_128_extend(tmp));

		self.query_storage::<H256>(block, &StorageKey(key))
	}

	/// Return the current block.
	fn current_block(&self, block: &BlockId<Block>) -> Option<ethereum::BlockV2> {
		self.query_storage::<ethereum::BlockV0>(
			block,
			&StorageKey(storage_prefix_build(b"Ethereum", b"CurrentBlock")),
		)
		.map(Into::into)
	}

	/// Return the current receipt.
	fn current_receipts(&self, block: &BlockId<Block>) -> Option<Vec<ethereum::ReceiptV3>> {
		self.query_storage::<Vec<ethereum::ReceiptV0>>(
			block,
			&StorageKey(storage_prefix_build(b"Ethereum", b"CurrentReceipts")),
		)
		.map(|receipts| {
			receipts
				.into_iter()
				.map(|r| {
					ethereum::ReceiptV3::Legacy(ethereum::EIP658ReceiptData {
						status_code: r.state_root.to_low_u64_be() as u8,
						used_gas: r.used_gas,
						logs_bloom: r.logs_bloom,
						logs: r.logs,
					})
				})
				.collect()
		})
	}

	/// Return the current transaction status.
	fn current_transaction_statuses(&self, block: &BlockId<Block>) -> Option<Vec<TransactionStatus>> {
		self.query_storage::<Vec<TransactionStatus>>(
			block,
			&StorageKey(storage_prefix_build(
				b"Ethereum",
				b"CurrentTransactionStatuses",
			)),
		)
	}

	/// Prior to eip-1559 there is no base fee.
	fn base_fee(&self, _block: &BlockId<Block>) -> Option<U256> {
		None
	}

	/// Prior to eip-1559 there is no elasticity.
	fn elasticity(&self, _block: &BlockId<Block>) -> Option<Permill> {
		None
	}

	fn is_eip1559(&self, _block: &BlockId<Block>) -> bool {
		false
	}
}
