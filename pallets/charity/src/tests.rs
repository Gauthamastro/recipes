use crate::*;
use balances;
use frame_support::{assert_err, assert_ok, impl_outer_event, impl_outer_origin, parameter_types};
use frame_system::{self as system, RawOrigin};
use sp_core::H256;
use sp_io;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill,
};

impl_outer_origin! {
	pub enum Origin for TestRuntime {}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TestRuntime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();

	pub const ExistentialDeposit: u64 = 1;
	pub const TransferFee: u64 = 0;
	pub const CreationFee: u64 = 0;
}
impl system::Trait for TestRuntime {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Index = u64;
	type Call = ();
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
	type AccountData = balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

impl balances::Trait for TestRuntime {
	type Balance = u64;
	type Event = TestEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = system::Module<TestRuntime>;
	type WeightInfo = ();
}

mod charity {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for TestRuntime {
		system<T>,
		charity<T>,
		balances<T>,
	}
}

impl Trait for TestRuntime {
	type Event = TestEvent;
	type Currency = balances::Module<Self>;
}

pub type System = system::Module<TestRuntime>;
pub type Balances = balances::Module<TestRuntime>;
pub type Charity = Module<TestRuntime>;

// An alternative to `ExtBuilder` which includes custom configuration
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = system::GenesisConfig::default()
		.build_storage::<TestRuntime>()
		.unwrap();

	balances::GenesisConfig::<TestRuntime> {
		// Provide some initial balances
		balances: vec![(1, 13), (2, 11), (3, 1), (4, 3), (5, 19)],
	}
		.assimilate_storage(&mut t)
		.unwrap();

	crate::GenesisConfig {}
		.assimilate_storage::<TestRuntime>(&mut t)
		.unwrap();

	let mut ext: sp_io::TestExternalities = t.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// Charity pot minimum balance is set
#[test]
fn pot_min_balance_is_set() {
	new_test_ext().execute_with(|| {
		assert_eq!(Charity::pot(), Balances::minimum_balance());
	})
}

/// Verifying correct behavior of boilerplate
#[test]
fn new_test_ext_behaves() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&1), 13);
	})
}

#[test]
fn donations_work() {
	new_test_ext().execute_with(|| {
		// User 1 donates 10 of her 13 tokens
		let original = Balances::free_balance(&1);
		let donation = 10;
		assert_ok!(Charity::donate(Origin::signed(1), donation));

		// Charity should have 10 tokens
		let new_pot_total = Balances::minimum_balance() + donation;
		assert_eq!(Charity::pot(), new_pot_total);

		// Donor should have 3 remaining
		assert_eq!(Balances::free_balance(&1), original - donation);

		// Check that the correct event is emitted
		let expected_event = TestEvent::charity(RawEvent::DonationReceived(1, donation, new_pot_total));
		assert!(System::events().iter().any(|a| a.event == expected_event));
	})
}

#[test]
fn cant_donate_too_much() {
	new_test_ext().execute_with(|| {
		// User 1 donates 20 toekns but only has 13
		assert_err!(
			Charity::donate(Origin::signed(1), 20),
			"Can't make donation"
		);
	})
}

#[test]
fn imbalances_work() {
	new_test_ext().execute_with(|| {
		let imb_amt = 5;
		let imb = balances::NegativeImbalance::new(imb_amt);
		Charity::on_nonzero_unbalanced(imb);

		let new_pot_total = imb_amt + Balances::minimum_balance();
		assert_eq!(Charity::pot(), new_pot_total);

		// Check that the correct event is emitted
		let expected_event = TestEvent::charity(RawEvent::ImbalanceAbsorbed(5, new_pot_total));

		assert!(System::events().iter().any(|a| a.event == expected_event));
	})
}

#[test]
fn allocating_works() {
	new_test_ext().execute_with(|| {
		// Charity acquires 10 tokens from user 1
		let donation = 10;
		assert_ok!(Charity::donate(Origin::signed(1), donation));

		// Charity allocates 5 tokens to user 2
		let alloc = 5;
		assert_ok!(Charity::allocate(RawOrigin::Root.into(), 2, alloc));

		// Check that the correct event is emitted
		let new_pot_total = Balances::minimum_balance() + donation - alloc;
		let expected_event = TestEvent::charity(RawEvent::FundsAllocated(2, 5, new_pot_total));
		assert!(System::events().iter().any(|a| a.event == expected_event));
	})
}
//TODO What if we try to allocate more funds than we have
#[test]
fn cant_allocate_too_much() {
	new_test_ext().execute_with(|| {
		// Charity acquires 10 tokens from user 1
		assert_ok!(Charity::donate(Origin::signed(1), 10));

		// Charity tries to allocates 20 tokens to user 2
		assert_err!(
			Charity::allocate(RawOrigin::Root.into(), 2, 20),
			"Can't make allocation"
		);
	})
}
