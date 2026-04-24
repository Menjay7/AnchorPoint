#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec, Map, token};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Initialized,
    Signers,       // Map<Address, u32>  (weight; 1 for M-of-N)
    Threshold,     // u32
    Mode,          // MultisigMode
    Recipient,
    Proposal(u64), // ProposalId -> Proposal
    NextProposalId,
    GovernanceProposal(u64), // GovernanceProposalId -> GovernanceProposal
    NextGovProposalId,
}

// ---------------------------------------------------------------------------
// Multisig mode
// ---------------------------------------------------------------------------
/// M-of-N: threshold = minimum number of signers required.
/// Weighted: threshold = minimum total weight required.
#[contracttype]
#[derive(Clone, PartialEq)]
pub enum MultisigMode {
    MofN,
    Weighted,
}

// ---------------------------------------------------------------------------
// Release proposal
// ---------------------------------------------------------------------------
#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub token: Address,
    pub approvals: Vec<Address>,
    pub expiration_ledger: u32,
    pub executed: bool,
}

// ---------------------------------------------------------------------------
// Governance proposal (add / remove signer)
// ---------------------------------------------------------------------------
#[contracttype]
#[derive(Clone, PartialEq)]
pub enum GovAction {
    AddSigner(Address, u32),   // address, weight
    RemoveSigner(Address),
}

#[contracttype]
#[derive(Clone)]
pub struct GovernanceProposal {
    pub action: GovAction,
    pub approvals: Vec<Address>,
    pub expiration_ledger: u32,
    pub executed: bool,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------
#[contract]
pub struct EscrowMultisig;

#[contractimpl]
impl EscrowMultisig {
    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// Initialize the contract.
    /// `signers`  – list of (address, weight) pairs; weight is ignored in MofN mode.
    /// `threshold` – number of signers (MofN) or total weight (Weighted).
    /// `mode`     – MultisigMode::MofN | MultisigMode::Weighted.
    /// `recipient` – address that receives released funds.
    pub fn initialize(
        e: Env,
        signers: Vec<(Address, u32)>,
        threshold: u32,
        mode: MultisigMode,
        recipient: Address,
    ) {
        if e.storage().instance().has(&DataKey::Initialized) {
            panic!("already initialized");
        }
        if signers.is_empty() {
            panic!("signers list cannot be empty");
        }
        if threshold == 0 {
            panic!("threshold must be > 0");
        }

        let mut signer_map: Map<Address, u32> = Map::new(&e);
        let mut total_weight: u32 = 0;
        for (addr, weight) in signers.iter() {
            if weight == 0 {
                panic!("signer weight must be > 0");
            }
            signer_map.set(addr.clone(), weight);
            total_weight += weight;
        }

        // Validate threshold is reachable
        match mode {
            MultisigMode::MofN => {
                if threshold > signer_map.len() {
                    panic!("threshold exceeds number of signers");
                }
            }
            MultisigMode::Weighted => {
                if threshold > total_weight {
                    panic!("threshold exceeds total weight");
                }
            }
        }

        e.storage().instance().set(&DataKey::Signers, &signer_map);
        e.storage().instance().set(&DataKey::Threshold, &threshold);
        e.storage().instance().set(&DataKey::Mode, &mode);
        e.storage().instance().set(&DataKey::Recipient, &recipient);
        e.storage().instance().set(&DataKey::Initialized, &true);
        e.storage().instance().set(&DataKey::NextProposalId, &0u64);
        e.storage().instance().set(&DataKey::NextGovProposalId, &0u64);
    }

    // -----------------------------------------------------------------------
    // Release proposals
    // -----------------------------------------------------------------------

    /// Create a new release proposal. Returns the proposal id.
    /// `proposer` must be a registered signer.
    /// `expiration_ledger` – ledger number after which the proposal is void.
    pub fn propose_release(
        e: Env,
        proposer: Address,
        token: Address,
        expiration_ledger: u32,
    ) -> u64 {
        proposer.require_auth();
        Self::assert_initialized(&e);
        Self::assert_is_signer(&e, &proposer);

        if expiration_ledger <= e.ledger().sequence() {
            panic!("expiration must be in the future");
        }

        let id = Self::next_proposal_id(&e);
        let mut approvals: Vec<Address> = Vec::new(&e);
        approvals.push_back(proposer.clone());

        let proposal = Proposal {
            token,
            approvals,
            expiration_ledger,
            executed: false,
        };
        e.storage().instance().set(&DataKey::Proposal(id), &proposal);
        id
    }

    /// Approve an existing release proposal.
    pub fn approve_release(e: Env, signer: Address, proposal_id: u64) {
        signer.require_auth();
        Self::assert_initialized(&e);
        Self::assert_is_signer(&e, &signer);

        let mut proposal: Proposal = e
            .storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .expect("proposal not found");

        if proposal.executed {
            panic!("proposal already executed");
        }
        if e.ledger().sequence() > proposal.expiration_ledger {
            panic!("proposal expired");
        }

        // Idempotent – skip if already approved
        for existing in proposal.approvals.iter() {
            if existing == signer {
                return;
            }
        }
        proposal.approvals.push_back(signer);
        e.storage().instance().set(&DataKey::Proposal(proposal_id), &proposal);
    }

    /// Execute a release proposal once threshold is met.
    pub fn execute_release(e: Env, proposal_id: u64) {
        Self::assert_initialized(&e);

        let mut proposal: Proposal = e
            .storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .expect("proposal not found");

        if proposal.executed {
            panic!("proposal already executed");
        }
        if e.ledger().sequence() > proposal.expiration_ledger {
            panic!("proposal expired");
        }

        Self::assert_threshold_met(&e, &proposal.approvals);

        let recipient: Address = e
            .storage()
            .instance()
            .get(&DataKey::Recipient)
            .expect("recipient not set");

        let token_client = token::Client::new(&e, &proposal.token);
        let balance = token_client.balance(&e.current_contract_address());
        if balance > 0 {
            token_client.transfer(&e.current_contract_address(), &recipient, &balance);
        }

        proposal.executed = true;
        e.storage().instance().set(&DataKey::Proposal(proposal_id), &proposal);
    }

    // -----------------------------------------------------------------------
    // Governance proposals (add / remove signer)
    // -----------------------------------------------------------------------

    /// Propose adding a new signer (or updating weight of existing one).
    pub fn propose_add_signer(
        e: Env,
        proposer: Address,
        new_signer: Address,
        weight: u32,
        expiration_ledger: u32,
    ) -> u64 {
        proposer.require_auth();
        Self::assert_initialized(&e);
        Self::assert_is_signer(&e, &proposer);

        if weight == 0 {
            panic!("weight must be > 0");
        }
        if expiration_ledger <= e.ledger().sequence() {
            panic!("expiration must be in the future");
        }

        let id = Self::next_gov_proposal_id(&e);
        let mut approvals: Vec<Address> = Vec::new(&e);
        approvals.push_back(proposer);

        let gov_proposal = GovernanceProposal {
            action: GovAction::AddSigner(new_signer, weight),
            approvals,
            expiration_ledger,
            executed: false,
        };
        e.storage().instance().set(&DataKey::GovernanceProposal(id), &gov_proposal);
        id
    }

    /// Propose removing an existing signer.
    pub fn propose_remove_signer(
        e: Env,
        proposer: Address,
        target: Address,
        expiration_ledger: u32,
    ) -> u64 {
        proposer.require_auth();
        Self::assert_initialized(&e);
        Self::assert_is_signer(&e, &proposer);

        if expiration_ledger <= e.ledger().sequence() {
            panic!("expiration must be in the future");
        }

        let id = Self::next_gov_proposal_id(&e);
        let mut approvals: Vec<Address> = Vec::new(&e);
        approvals.push_back(proposer);

        let gov_proposal = GovernanceProposal {
            action: GovAction::RemoveSigner(target),
            approvals,
            expiration_ledger,
            executed: false,
        };
        e.storage().instance().set(&DataKey::GovernanceProposal(id), &gov_proposal);
        id
    }

    /// Approve a governance proposal.
    pub fn approve_governance(e: Env, signer: Address, gov_proposal_id: u64) {
        signer.require_auth();
        Self::assert_initialized(&e);
        Self::assert_is_signer(&e, &signer);

        let mut gov_proposal: GovernanceProposal = e
            .storage()
            .instance()
            .get(&DataKey::GovernanceProposal(gov_proposal_id))
            .expect("governance proposal not found");

        if gov_proposal.executed {
            panic!("governance proposal already executed");
        }
        if e.ledger().sequence() > gov_proposal.expiration_ledger {
            panic!("governance proposal expired");
        }

        for existing in gov_proposal.approvals.iter() {
            if existing == signer {
                return;
            }
        }
        gov_proposal.approvals.push_back(signer);
        e.storage().instance().set(&DataKey::GovernanceProposal(gov_proposal_id), &gov_proposal);
    }

    /// Execute a governance proposal once threshold is met.
    pub fn execute_governance(e: Env, gov_proposal_id: u64) {
        Self::assert_initialized(&e);

        let mut gov_proposal: GovernanceProposal = e
            .storage()
            .instance()
            .get(&DataKey::GovernanceProposal(gov_proposal_id))
            .expect("governance proposal not found");

        if gov_proposal.executed {
            panic!("governance proposal already executed");
        }
        if e.ledger().sequence() > gov_proposal.expiration_ledger {
            panic!("governance proposal expired");
        }

        Self::assert_threshold_met(&e, &gov_proposal.approvals);

        let mut signer_map: Map<Address, u32> = e
            .storage()
            .instance()
            .get(&DataKey::Signers)
            .expect("not initialized");

        match gov_proposal.action.clone() {
            GovAction::AddSigner(addr, weight) => {
                signer_map.set(addr, weight);
            }
            GovAction::RemoveSigner(addr) => {
                signer_map.remove(addr);
                if signer_map.is_empty() {
                    panic!("cannot remove last signer");
                }
                // Ensure threshold is still reachable after removal
                let threshold: u32 = e
                    .storage()
                    .instance()
                    .get(&DataKey::Threshold)
                    .expect("threshold not set");
                let mode: MultisigMode = e
                    .storage()
                    .instance()
                    .get(&DataKey::Mode)
                    .expect("mode not set");
                match mode {
                    MultisigMode::MofN => {
                        if threshold > signer_map.len() {
                            panic!("removal would make threshold unreachable");
                        }
                    }
                    MultisigMode::Weighted => {
                        let total: u32 = signer_map.values().iter().sum();
                        if threshold > total {
                            panic!("removal would make threshold unreachable");
                        }
                    }
                }
            }
        }

        e.storage().instance().set(&DataKey::Signers, &signer_map);
        gov_proposal.executed = true;
        e.storage().instance().set(&DataKey::GovernanceProposal(gov_proposal_id), &gov_proposal);
    }

    // -----------------------------------------------------------------------
    // View functions
    // -----------------------------------------------------------------------

    pub fn get_signers(e: Env) -> Map<Address, u32> {
        e.storage().instance().get(&DataKey::Signers).unwrap_or(Map::new(&e))
    }

    pub fn get_threshold(e: Env) -> u32 {
        e.storage().instance().get(&DataKey::Threshold).unwrap_or(0)
    }

    pub fn get_mode(e: Env) -> MultisigMode {
        e.storage().instance().get(&DataKey::Mode).expect("not initialized")
    }

    pub fn get_recipient(e: Env) -> Address {
        e.storage().instance().get(&DataKey::Recipient).expect("recipient not set")
    }

    pub fn get_proposal(e: Env, proposal_id: u64) -> Proposal {
        e.storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .expect("proposal not found")
    }

    pub fn get_governance_proposal(e: Env, gov_proposal_id: u64) -> GovernanceProposal {
        e.storage()
            .instance()
            .get(&DataKey::GovernanceProposal(gov_proposal_id))
            .expect("governance proposal not found")
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn assert_initialized(e: &Env) {
        if !e.storage().instance().has(&DataKey::Initialized) {
            panic!("not initialized");
        }
    }

    fn assert_is_signer(e: &Env, addr: &Address) {
        let signer_map: Map<Address, u32> = e
            .storage()
            .instance()
            .get(&DataKey::Signers)
            .expect("not initialized");
        if !signer_map.contains_key(addr.clone()) {
            panic!("not a registered signer");
        }
    }

    /// Check that the approvals list satisfies the configured threshold.
    fn assert_threshold_met(e: &Env, approvals: &Vec<Address>) {
        let signer_map: Map<Address, u32> = e
            .storage()
            .instance()
            .get(&DataKey::Signers)
            .expect("not initialized");
        let threshold: u32 = e
            .storage()
            .instance()
            .get(&DataKey::Threshold)
            .expect("threshold not set");
        let mode: MultisigMode = e
            .storage()
            .instance()
            .get(&DataKey::Mode)
            .expect("mode not set");

        match mode {
            MultisigMode::MofN => {
                if approvals.len() < threshold {
                    panic!("threshold not met");
                }
            }
            MultisigMode::Weighted => {
                let mut total: u32 = 0;
                for addr in approvals.iter() {
                    if let Some(w) = signer_map.get(addr) {
                        total += w;
                    }
                }
                if total < threshold {
                    panic!("weighted threshold not met");
                }
            }
        }
    }

    fn next_proposal_id(e: &Env) -> u64 {
        let id: u64 = e
            .storage()
            .instance()
            .get(&DataKey::NextProposalId)
            .unwrap_or(0);
        e.storage().instance().set(&DataKey::NextProposalId, &(id + 1));
        id
    }

    fn next_gov_proposal_id(e: &Env) -> u64 {
        let id: u64 = e
            .storage()
            .instance()
            .get(&DataKey::NextGovProposalId)
            .unwrap_or(0);
        e.storage().instance().set(&DataKey::NextGovProposalId, &(id + 1));
        id
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        token::StellarAssetClient,
        Address, Env, Vec,
    };

    fn setup_mofn(e: &Env, n: u32, threshold: u32) -> (Vec<Address>, Address, Address) {
        let mut signers_vec: Vec<(Address, u32)> = Vec::new(e);
        let mut addr_vec: Vec<Address> = Vec::new(e);
        for _ in 0..n {
            let a = Address::generate(e);
            signers_vec.push_back((a.clone(), 1u32));
            addr_vec.push_back(a);
        }
        let recipient = Address::generate(e);
        let contract_id = e.register(EscrowMultisig, ());
        let client = EscrowMultisigClient::new(e, &contract_id);
        client.initialize(&signers_vec, &threshold, &MultisigMode::MofN, &recipient);
        (addr_vec, recipient, contract_id)
    }

    // --- M-of-N release ---

    #[test]
    fn test_mofn_release() {
        let e = Env::default();
        e.mock_all_auths();

        let (signers, recipient, contract_id) = setup_mofn(&e, 5, 3);
        let client = EscrowMultisigClient::new(&e, &contract_id);

        let admin = Address::generate(&e);
        let token_id = e.register_stellar_asset_contract_v2(admin.clone()).address();
        let sac = StellarAssetClient::new(&e, &token_id);
        sac.mint(&contract_id, &1000i128);
        let token_client = token::Client::new(&e, &token_id);

        // Propose
        let pid = client.propose_release(&signers.get(0).unwrap(), &token_id, &(e.ledger().sequence() + 100));

        // Approve by 2 more signers
        client.approve_release(&signers.get(1).unwrap(), &pid);
        client.approve_release(&signers.get(2).unwrap(), &pid);

        // Execute
        client.execute_release(&pid);

        assert_eq!(token_client.balance(&recipient), 1000);
        assert_eq!(token_client.balance(&contract_id), 0);
    }

    #[test]
    #[should_panic(expected = "threshold not met")]
    fn test_mofn_threshold_not_met() {
        let e = Env::default();
        e.mock_all_auths();

        let (signers, _, contract_id) = setup_mofn(&e, 3, 2);
        let client = EscrowMultisigClient::new(&e, &contract_id);

        let token_id = Address::generate(&e);
        let pid = client.propose_release(&signers.get(0).unwrap(), &token_id, &(e.ledger().sequence() + 100));
        // Only 1 approval (the proposer) – threshold is 2
        client.execute_release(&pid);
    }

    // --- Weighted release ---

    #[test]
    fn test_weighted_release() {
        let e = Env::default();
        e.mock_all_auths();

        let a = Address::generate(&e);
        let b = Address::generate(&e);
        let c = Address::generate(&e);
        let recipient = Address::generate(&e);

        // weights: a=3, b=2, c=1  threshold=4
        let mut signers_vec: Vec<(Address, u32)> = Vec::new(&e);
        signers_vec.push_back((a.clone(), 3u32));
        signers_vec.push_back((b.clone(), 2u32));
        signers_vec.push_back((c.clone(), 1u32));

        let contract_id = e.register(EscrowMultisig, ());
        let client = EscrowMultisigClient::new(&e, &contract_id);
        client.initialize(&signers_vec, &4u32, &MultisigMode::Weighted, &recipient);

        let admin = Address::generate(&e);
        let token_id = e.register_stellar_asset_contract_v2(admin.clone()).address();
        let sac = StellarAssetClient::new(&e, &token_id);
        sac.mint(&contract_id, &500i128);
        let token_client = token::Client::new(&e, &token_id);

        // a proposes (weight 3), b approves (weight 2) → total 5 >= 4
        let pid = client.propose_release(&a, &token_id, &(e.ledger().sequence() + 100));
        client.approve_release(&b, &pid);
        client.execute_release(&pid);

        assert_eq!(token_client.balance(&recipient), 500);
    }

    #[test]
    #[should_panic(expected = "weighted threshold not met")]
    fn test_weighted_threshold_not_met() {
        let e = Env::default();
        e.mock_all_auths();

        let a = Address::generate(&e);
        let b = Address::generate(&e);
        let recipient = Address::generate(&e);

        let mut signers_vec: Vec<(Address, u32)> = Vec::new(&e);
        signers_vec.push_back((a.clone(), 1u32));
        signers_vec.push_back((b.clone(), 1u32));

        let contract_id = e.register(EscrowMultisig, ());
        let client = EscrowMultisigClient::new(&e, &contract_id);
        client.initialize(&signers_vec, &2u32, &MultisigMode::Weighted, &recipient);

        let token_id = Address::generate(&e);
        // Only a proposes (weight 1) – threshold 2 not met
        let pid = client.propose_release(&a, &token_id, &(e.ledger().sequence() + 100));
        client.execute_release(&pid);
    }

    // --- Expiration ---

    #[test]
    #[should_panic(expected = "proposal expired")]
    fn test_proposal_expiration() {
        let e = Env::default();
        e.mock_all_auths();

        let (signers, _, contract_id) = setup_mofn(&e, 3, 2);
        let client = EscrowMultisigClient::new(&e, &contract_id);

        let token_id = Address::generate(&e);
        // Expire at current ledger (already past)
        let pid = client.propose_release(&signers.get(0).unwrap(), &token_id, &(e.ledger().sequence() + 1));

        // Advance ledger past expiration
        e.ledger().with_mut(|li| li.sequence_number += 10);

        client.approve_release(&signers.get(1).unwrap(), &pid);
    }

    #[test]
    #[should_panic(expected = "proposal expired")]
    fn test_execute_expired_proposal() {
        let e = Env::default();
        e.mock_all_auths();

        let (signers, _, contract_id) = setup_mofn(&e, 3, 2);
        let client = EscrowMultisigClient::new(&e, &contract_id);

        let token_id = Address::generate(&e);
        let pid = client.propose_release(&signers.get(0).unwrap(), &token_id, &(e.ledger().sequence() + 5));
        client.approve_release(&signers.get(1).unwrap(), &pid);

        // Advance past expiration
        e.ledger().with_mut(|li| li.sequence_number += 10);
        client.execute_release(&pid);
    }

    // --- Governance: add signer ---

    #[test]
    fn test_governance_add_signer() {
        let e = Env::default();
        e.mock_all_auths();

        let (signers, _, contract_id) = setup_mofn(&e, 3, 2);
        let client = EscrowMultisigClient::new(&e, &contract_id);

        let new_signer = Address::generate(&e);
        let gid = client.propose_add_signer(
            &signers.get(0).unwrap(),
            &new_signer,
            &1u32,
            &(e.ledger().sequence() + 100),
        );
        client.approve_governance(&signers.get(1).unwrap(), &gid);
        client.execute_governance(&gid);

        let signer_map = client.get_signers();
        assert!(signer_map.contains_key(new_signer));
    }

    // --- Governance: remove signer ---

    #[test]
    fn test_governance_remove_signer() {
        let e = Env::default();
        e.mock_all_auths();

        let (signers, _, contract_id) = setup_mofn(&e, 3, 2);
        let client = EscrowMultisigClient::new(&e, &contract_id);

        let to_remove = signers.get(2).unwrap();
        let gid = client.propose_remove_signer(
            &signers.get(0).unwrap(),
            &to_remove,
            &(e.ledger().sequence() + 100),
        );
        client.approve_governance(&signers.get(1).unwrap(), &gid);
        client.execute_governance(&gid);

        let signer_map = client.get_signers();
        assert!(!signer_map.contains_key(to_remove));
    }

    #[test]
    #[should_panic]
    fn test_governance_remove_makes_threshold_unreachable() {
        let e = Env::default();
        e.mock_all_auths();

        // 2 signers, threshold 2 – removing one makes it unreachable
        let (signers, _, contract_id) = setup_mofn(&e, 2, 2);
        let client = EscrowMultisigClient::new(&e, &contract_id);

        let to_remove = signers.get(1).unwrap();
        let gid = client.propose_remove_signer(
            &signers.get(0).unwrap(),
            &to_remove,
            &(e.ledger().sequence() + 100),
        );
        client.approve_governance(&signers.get(0).unwrap(), &gid);
        client.execute_governance(&gid);
    }

    // --- Duplicate approval is idempotent ---

    #[test]
    fn test_duplicate_approval_ignored() {
        let e = Env::default();
        e.mock_all_auths();

        let (signers, _, contract_id) = setup_mofn(&e, 3, 2);
        let client = EscrowMultisigClient::new(&e, &contract_id);

        let token_id = Address::generate(&e);
        let pid = client.propose_release(&signers.get(0).unwrap(), &token_id, &(e.ledger().sequence() + 100));
        client.approve_release(&signers.get(0).unwrap(), &pid); // duplicate
        client.approve_release(&signers.get(0).unwrap(), &pid); // duplicate again

        let proposal = client.get_proposal(&pid);
        // Should still only have 1 approval entry
        assert_eq!(proposal.approvals.len(), 1);
    }

    // --- Non-signer cannot propose ---

    #[test]
    #[should_panic(expected = "not a registered signer")]
    fn test_non_signer_cannot_propose() {
        let e = Env::default();
        e.mock_all_auths();

        let (_, _, contract_id) = setup_mofn(&e, 3, 2);
        let client = EscrowMultisigClient::new(&e, &contract_id);

        let outsider = Address::generate(&e);
        let token_id = Address::generate(&e);
        client.propose_release(&outsider, &token_id, &(e.ledger().sequence() + 100));
    }
}
