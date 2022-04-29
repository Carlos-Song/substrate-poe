#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	// The struct on which we build all of our Pallet logic.
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// 因为这个 pallet 会发出事件，所以它取决于运行时对事件的定义。
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// 用于约束存证的哈希的最大字节数
		type MaxBytesInHash: Get<u32>;
	}


	/// Pallets 使用事件通知用户何时进行重要的更改
	/// 事件文档应该以一个数组结束，该数组为参数提供描述性名称
	/// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// 当凭证被声明创建时，发出一个事件. [who, claim]
		ClaimCreated(T::AccountId, BoundedVec<u8, T::MaxBytesInHash>),
		/// 当一个凭证声明被持有者撤销时，发出一个事件. [who, claim]
		ClaimRevoked(T::AccountId, BoundedVec<u8, T::MaxBytesInHash>),
		/// 当发送者转移持有权时，发出一个事件. [from, to, claim]
		ClaimTransfered(T::AccountId, T::AccountId, BoundedVec<u8, T::MaxBytesInHash>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// 当前凭证已被声明.
		ProofAlreadyClaimed,
		/// 当前凭证不存在，无法被更改。
		NoSuchProof,
		/// 存证已经被其他持有者声明，所以调用者无法进行更改
		NotProofOwner,
	}

	#[pallet::storage]
    /// Maps each proof to its owner and block number when the proof was made
    pub(super) type Proofs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BoundedVec<u8, T::MaxBytesInHash>,
        (T::AccountId, T::BlockNumber),
        OptionQuery,
    >;

	// 可调度函数允许用户与 pallet 交互并调用状态更改。
	// 这些函数具体化为 extrinsics(外部交易)，通常被比作事务
	// 可调度函数必须用权重 weight 注释，并且必须返回调度结果。
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1_000)]
        pub fn create_claim(
            origin: OriginFor<T>,
            proof: BoundedVec<u8, T::MaxBytesInHash>,
        ) -> DispatchResult {
            // 检查 extrinsics 是否已签名，并找到签名者.
            // 如果 extrinsics 未进行名，此函数将返回一个错误
            // https://docs.substrate.io/v3/runtime/origins
            let sender = ensure_signed(origin)?;

            // 验证指定的存证是否尚未声明。
            ensure!(!Proofs::<T>::contains_key(&proof), Error::<T>::ProofAlreadyClaimed);

            // 从 FRAME System pallet 获取区块号.
            let current_block = <frame_system::Pallet<T>>::block_number();

			// 存储存证中的 发送者 和 区块号
            Proofs::<T>::insert(&proof, (&sender, current_block));

     		// 发出一个存证被创建的事件
            Self::deposit_event(Event::ClaimCreated(sender, proof));

            Ok(())
        }


		#[pallet::weight(1_000)]
        pub fn transfer_claim(
            origin: OriginFor<T>,
            account: T::AccountId,
            proof: BoundedVec<u8, T::MaxBytesInHash>,
        ) -> DispatchResult {
            // 检查 extrinsics 是否已签名，并找到签名者.
            // 如果 extrinsics 未进行名，此函数将返回一个错误
            let sender = ensure_signed(origin)?;

			// 验证指定的存证是否被声明。
			ensure!(Proofs::<T>::contains_key(&proof), Error::<T>::NoSuchProof);

            // 获取创建者信息.
            // Panic 条件: 无法设置一个 `None` 持有者, 因此总是需要使用 unwrap 包裹.
            let (owner, _) = Proofs::<T>::get(&proof).expect("All proofs must have an owner!");

            // 验证函数调用的发起者是否拥有存证的所有权.
            ensure!(sender == owner, Error::<T>::NotProofOwner);

			// 从区块中转移存证所有权
            Proofs::<T>::mutate(&proof, |values| {
				match values {
					Some(value) => {
						value.0 = account.clone();
					},
					// 不做任何事情，因为 line 101 已经进行过存证存在性校验
					_ =>  (),
				};
			});

     		// 发出一个存证所有权转移的事件
            Self::deposit_event(Event::ClaimTransfered(sender, account, proof));

            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn revoke_claim(
            origin: OriginFor<T>,
            proof: BoundedVec<u8, T::MaxBytesInHash>,
        ) -> DispatchResult {
          	// 检查 extrinsics 是否已签名，并找到签名者.
            // 如果 extrinsics 未进行名，此函数将返回一个错误
            // https://docs.substrate.io/v3/runtime/origins
            let sender = ensure_signed(origin)?;

			// 验证指定的存证是否被声明。
            ensure!(Proofs::<T>::contains_key(&proof), Error::<T>::NoSuchProof);

            // 获取创建者信息.
            // Panic 条件: 无法设置一个 `None` 持有者, 因此总是需要使用 unwrap 包裹.
            let (owner, _) = Proofs::<T>::get(&proof).expect("All proofs must have an owner!");

            // 验证函数调用的发起者是否拥有存证的所有权.
            ensure!(sender == owner, Error::<T>::NotProofOwner);

            // 从区块中移除存证声明.
            Proofs::<T>::remove(&proof);

       		// 发出一个存证被抹除的事件
            Self::deposit_event(Event::ClaimRevoked(sender, proof));
            Ok(())
        }
    }
}
