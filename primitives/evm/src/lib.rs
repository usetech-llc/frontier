// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2020 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

mod precompile;

use codec::{Decode, Encode};
pub use evm::ExitReason;
use impl_trait_for_tuples::impl_for_tuples;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::{H160, U256};
use sp_std::vec::Vec;

pub use evm::backend::{Basic as Account, Log};
pub use precompile::{
	Context, ExitError, ExitSucceed, LinearCostPrecompile, Precompile, PrecompileFailure,
	PrecompileOutput, PrecompileResult, PrecompileSet,
};

#[derive(Clone, Eq, PartialEq, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
/// External input from the transaction.
pub struct Vicinity {
	/// Current transaction gas price.
	pub gas_price: U256,
	/// Origin of the transaction.
	pub origin: H160,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct ExecutionInfo<T> {
	pub exit_reason: ExitReason,
	pub value: T,
	pub used_gas: U256,
	pub logs: Vec<Log>,
}

pub type CallInfo = ExecutionInfo<Vec<u8>>;
pub type CreateInfo = ExecutionInfo<H160>;

#[derive(Clone, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub enum CallOrCreateInfo {
	Call(CallInfo),
	Create(CreateInfo),
}

#[derive(Debug)]
pub enum WithdrawReason {
	Call { target: H160, input: Vec<u8> },
	Create,
	Create2,
}

// TODO: Refactor into something less specific
pub trait TransactionValidityHack<CrossAccountId> {
	fn who_pays_fee(origin: H160, reason: &WithdrawReason) -> Option<CrossAccountId>;
}

impl<CrossAccountId> TransactionValidityHack<CrossAccountId> for () {
	fn who_pays_fee(_origin: H160, _reason: &WithdrawReason) -> Option<CrossAccountId> {
		None
	}
}

#[impl_for_tuples(1, 12)]
impl<CrossAccountId> TransactionValidityHack<CrossAccountId> for Tuple {
	fn who_pays_fee(origin: H160, reason: &WithdrawReason) -> Option<CrossAccountId> {
		for_tuples!(#(
			if let Some(who) = Tuple::who_pays_fee(origin, reason) {
				return Some(who);
			}
		)*);
		None
	}
}
