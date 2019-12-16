// Copyright 2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Storage migrations for srml-staking.

/// Indicator of a version of a storage layout.
pub type VersionNumber = u32;

// the current expected version of the storage
pub const CURRENT_VERSION: VersionNumber = 2;

#[cfg(any(test, feature = "migrate"))]
mod inner {
	use crate::{Store, Module, Trait, ValidatorInfoForEra, SessionInterface};
	use frame_support::{StorageLinkedMap, StorageValue, StorageMap, StoragePrefixedMap};
	use sp_std::vec::Vec;
	use super::{CURRENT_VERSION, VersionNumber};

	// the minimum supported version of the migration logic.
	const MIN_SUPPORTED_VERSION: VersionNumber = 0;

	// migrate storage from v0 to v1.
	//
	// this upgrades the `Nominators` linked_map value type from `Vec<T::AccountId>` to
	// `Option<Nominations<T::AccountId>>`
	pub fn to_v1<T: Trait>(version: &mut VersionNumber) {
		if *version != 0 { return }
		*version += 1;

		let now = <Module<T>>::current_era();
		let res = <Module<T> as Store>::Nominators::translate::<T::AccountId, Vec<T::AccountId>, _, _>(
			|key| key,
			|targets| crate::Nominations {
				targets,
				submitted_in: now,
				suppressed: false,
			},
		);

		if let Err(e) = res {
			frame_support::print("Encountered error in migration of Staking::Nominators map.");
			if e.is_none() {
				frame_support::print("Staking::Nominators map reinitialized");
			}
		}

		frame_support::print("Finished migrating Staking storage to v1.");
	}

	// migrate storage from v1 to v2.
	//
	// * populate `EraStartSessionIndex` storage with only the current era start
	// * populate `ValidatorForEra` storage with information from:
	//   * `Trait::SessionInterface::validators` for validator set
	//   * `Stakers` for exposure of validators
	//   * `Validators` for prefs of validators
	// * populate `SlotStakeForEra` storage with information from `SlotStake`
	// * remove `CurrentEraStartSessionIndex` storage
	// * remove `CurrentElected` storage
	// * remove `SlotStake` storage
	pub fn from_v1_to_v2<T: Trait>(version: &mut VersionNumber) {
		if *version != 1 { return }
		*version += 1;

		let current_era = <Module<T>>::current_era();
		let current_validator_infos = T::SessionInterface::validators().into_iter()
			.map(|validator| ValidatorInfoForEra {
				prefs: <Module<T>>::validators(&validator),
				exposure: <Module<T> as Store>::Stakers::get(&validator),
				stash: validator,
			})
			.collect::<Vec<_>>();
		let current_slot_stake = <Module<T> as Store>::SlotStake::get();

		<Module<T> as Store>::ValidatorForEra::insert(&current_era, current_validator_infos);
		<Module<T> as Store>::SlotStakeForEra::insert(&current_era, current_slot_stake);
		<Module<T> as Store>::EraStartSessionIndex::put(sp_std::vec![current_era]);

		<Module<T> as Store>::Stakers::remove_all();
		<Module<T> as Store>::CurrentElected::kill();
		<Module<T> as Store>::SlotStake::kill();

		frame_support::print("Finished migrating Staking storage to v2.");
	}

	pub(super) fn perform_migrations<T: Trait>() {
		<Module<T> as Store>::StorageVersion::mutate(|version| {
			if *version < MIN_SUPPORTED_VERSION {
				frame_support::print("Cannot migrate staking storage because version is less than\
					minimum.");
				frame_support::print(*version);
				return
			}

			if *version == CURRENT_VERSION { return }

			to_v1::<T>(version);
			from_v1_to_v2::<T>(version);
		});
	}
}

#[cfg(not(any(test, feature = "migrate")))]
mod inner {
	pub(super) fn perform_migrations<T>() { }
}

/// Perform all necessary storage migrations to get storage into the expected stsate for current
/// logic. No-op if fully upgraded.
pub(crate) fn perform_migrations<T: crate::Trait>() {
	inner::perform_migrations::<T>();
}
