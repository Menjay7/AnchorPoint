#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Bytes, BytesN, xdr::ToXdr, Vec};

#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KycTier {
    Basic = 1,
    Pro = 2,
}

#[contracttype]
pub struct KycData {
    pub tier: KycTier,
    pub expires_at: u64,
    pub verifier_version: u32,
}

#[contracttype]
pub enum DataKey {
    Admin,
    VerifierPubKey,
    VerifierVersion, // u32
    UserKyc(Address), // KycData
}

#[contract]
pub struct KycVerifier;

#[contractimpl]
impl KycVerifier {
    pub fn initialize(env: Env, admin: Address, verifier_pubkey: BytesN<32>) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::VerifierPubKey, &verifier_pubkey);
        env.storage().instance().set(&DataKey::VerifierVersion, &1u32);
    }

    pub fn set_kyc_status(env: Env, user: Address, tier: KycTier, signature: BytesN<64>, expires_at: u64) {
        Self::set_kyc_internal(&env, user, tier, signature, expires_at);
    }

    pub fn batch_set_kyc_status(
        env: Env,
        users: Vec<Address>,
        tiers: Vec<KycTier>,
        signatures: Vec<BytesN<64>>,
        expires_at: Vec<u64>,
    ) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        
        assert!(users.len() == tiers.len() && users.len() == signatures.len() && users.len() == expires_at.len(), "length mismatch");
        for i in 0..users.len() {
            Self::set_kyc_internal(&env, users.get(i).unwrap(), tiers.get(i).unwrap(), signatures.get(i).unwrap(), expires_at.get(i).unwrap());
        }
    }

    fn set_kyc_internal(env: &Env, user: Address, tier: KycTier, signature: BytesN<64>, expires_at: u64) {
        let current_time = env.ledger().timestamp();
        assert!(expires_at > current_time, "proof expired");

        let mut data = Bytes::new(env);
        data.append(&user.clone().to_xdr(env));
        data.append(&Bytes::from_slice(env, &(expires_at as u128).to_be_bytes()));
        
        let pubkey: BytesN<32> = env.storage().instance().get(&DataKey::VerifierPubKey).unwrap();
        
        env.crypto().ed25519_verify(&pubkey, &data, &signature);

        let verifier_version: u32 = env.storage().instance().get(&DataKey::VerifierVersion).unwrap();
        
        let kyc_data = KycData {
            tier,
            expires_at,
            verifier_version,
        };

        env.storage().persistent().set(&DataKey::UserKyc(user.clone()), &kyc_data);
        env.events().publish((symbol_short!("kyc_set"), user, tier as u32), expires_at);
    }

    pub fn is_kyc_valid(env: Env, user: Address) -> bool {
        let current_time = env.ledger().timestamp();
        let current_version: u32 = env.storage().instance().get(&DataKey::VerifierVersion).unwrap();

        match env.storage().persistent().get::<_, KycData>(&DataKey::UserKyc(user)) {
            Some(data) => data.expires_at > current_time && data.verifier_version == current_version,
            None => false,
        }
    }

    pub fn get_tier_limit(env: Env, tier: KycTier) -> u128 {
        match tier {
            KycTier::Basic => 1_000_000_000, // 1000 USDC e.g. (7 decimals)
            KycTier::Pro => 100_000_000_000, // 100k USDC
        }
    }

    pub fn update_verifier(env: Env, new_pubkey: BytesN<32>) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        env.storage().instance().set(&DataKey::VerifierPubKey, &new_pubkey);
        
        let mut version: u32 = env.storage().instance().get(&DataKey::VerifierVersion).unwrap_or(1);
        version += 1;
        env.storage().instance().set(&DataKey::VerifierVersion, &version);
    }
}
