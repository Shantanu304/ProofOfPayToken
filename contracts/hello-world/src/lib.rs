#![allow(non_snake_case)]
#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, log, symbol_short, Address, Env, String, Symbol,
};

// Structure to track the investment pool status
#[contracttype]
#[derive(Clone)]
pub struct PoolStatus {
    pub total_invested: i128,    // Total tokens invested in the pool
    pub total_investors: u64,    // Number of unique investors
    pub active_proposals: u64,   // Number of active proposals
    pub executed_proposals: u64, // Number of executed proposals
}

// Structure for investment proposals
#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub proposal_id: u64,
    pub title: String,
    pub description: String,
    pub amount: i128,       // Amount requested from pool
    pub votes_for: u64,     // Number of votes in favor
    pub votes_against: u64, // Number of votes against
    pub is_executed: bool,  // Whether proposal has been executed
    pub created_at: u64,    // Timestamp of creation
}

// Structure to track individual investor data
#[contracttype]
#[derive(Clone)]
pub struct Investor {
    pub address: Address,
    pub invested_amount: i128, // Total amount invested
    pub voting_power: u64,     // Voting power based on investment
    pub joined_at: u64,        // Timestamp when joined
}

// Constants for storage keys
const POOL_STATUS: Symbol = symbol_short!("POOL_STAT");
const PROPOSAL_COUNT: Symbol = symbol_short!("PROP_CNT");

// Mapping for proposals
#[contracttype]
pub enum ProposalBook {
    Proposal(u64),
}

// Mapping for investors
#[contracttype]
pub enum InvestorBook {
    Investor(Address),
}

#[contract]
pub struct InvestorClubContract;

#[contractimpl]
impl InvestorClubContract {
    // Function 1: Join the investment pool by investing tokens
    pub fn invest(env: Env, investor: Address, amount: i128) -> bool {
        investor.require_auth();

        if amount <= 0 {
            log!(&env, "Investment amount must be positive");
            panic!("Investment amount must be positive");
        }

        let time = env.ledger().timestamp();
        let mut pool_status = Self::view_pool_status(env.clone());
        let mut investor_data = Self::view_investor(env.clone(), investor.clone());

        // Check if this is a new investor
        let is_new_investor = investor_data.invested_amount == 0;

        // Update investor data
        investor_data.address = investor.clone();
        investor_data.invested_amount += amount;
        investor_data.voting_power = (investor_data.invested_amount / 100) as u64; // 1 vote per 100 tokens

        if is_new_investor {
            investor_data.joined_at = time;
            pool_status.total_investors += 1;
        }

        // Update pool status
        pool_status.total_invested += amount;

        // Store updated data
        env.storage()
            .instance()
            .set(&InvestorBook::Investor(investor.clone()), &investor_data);
        env.storage().instance().set(&POOL_STATUS, &pool_status);
        env.storage().instance().extend_ttl(5000, 5000);

        log!(
            &env,
            "Investment successful! Amount: {}, Total voting power: {}",
            amount,
            investor_data.voting_power
        );
        true
    }

    // Function 2: Create a new investment proposal
    pub fn create_proposal(
        env: Env,
        creator: Address,
        title: String,
        description: String,
        amount: i128,
    ) -> u64 {
        creator.require_auth();

        // Verify creator is an investor
        let creator_data = Self::view_investor(env.clone(), creator.clone());
        if creator_data.invested_amount <= 0 {
            log!(&env, "Only investors can create proposals");
            panic!("Only investors can create proposals");
        }

        let mut proposal_count: u64 = env.storage().instance().get(&PROPOSAL_COUNT).unwrap_or(0);
        proposal_count += 1;

        let time = env.ledger().timestamp();
        let mut pool_status = Self::view_pool_status(env.clone());

        // Create new proposal
        let proposal = Proposal {
            proposal_id: proposal_count,
            title,
            description,
            amount,
            votes_for: 0,
            votes_against: 0,
            is_executed: false,
            created_at: time,
        };

        // Update pool status
        pool_status.active_proposals += 1;

        // Store proposal and updated data
        env.storage()
            .instance()
            .set(&ProposalBook::Proposal(proposal_count), &proposal);
        env.storage()
            .instance()
            .set(&PROPOSAL_COUNT, &proposal_count);
        env.storage().instance().set(&POOL_STATUS, &pool_status);
        env.storage().instance().extend_ttl(5000, 5000);

        log!(&env, "Proposal created with ID: {}", proposal_count);
        proposal_count
    }

    // Function 3: Vote on a proposal
    pub fn vote_on_proposal(env: Env, voter: Address, proposal_id: u64, vote_for: bool) {
        voter.require_auth();

        // Verify voter is an investor
        let voter_data = Self::view_investor(env.clone(), voter.clone());
        if voter_data.invested_amount <= 0 {
            log!(&env, "Only investors can vote");
            panic!("Only investors can vote");
        }

        // Get proposal
        let mut proposal = Self::view_proposal(env.clone(), proposal_id);

        if proposal.proposal_id == 0 {
            log!(&env, "Proposal not found");
            panic!("Proposal not found");
        }

        if proposal.is_executed {
            log!(&env, "Proposal already executed");
            panic!("Proposal already executed");
        }

        // Add votes based on voting power
        if vote_for {
            proposal.votes_for += voter_data.voting_power;
        } else {
            proposal.votes_against += voter_data.voting_power;
        }

        // Store updated proposal
        env.storage()
            .instance()
            .set(&ProposalBook::Proposal(proposal_id), &proposal);
        env.storage().instance().extend_ttl(5000, 5000);

        log!(
            &env,
            "Vote recorded! Proposal ID: {}, Vote For: {}",
            proposal_id,
            vote_for
        );
    }

    // Function 4: Execute an approved proposal
    pub fn execute_proposal(env: Env, executor: Address, proposal_id: u64) {
        executor.require_auth();

        let mut proposal = Self::view_proposal(env.clone(), proposal_id);
        let pool_status = Self::view_pool_status(env.clone());

        if proposal.proposal_id == 0 {
            log!(&env, "Proposal not found");
            panic!("Proposal not found");
        }

        if proposal.is_executed {
            log!(&env, "Proposal already executed");
            panic!("Proposal already executed");
        }

        // Check if proposal is approved (more votes for than against)
        if proposal.votes_for <= proposal.votes_against {
            log!(&env, "Proposal not approved");
            panic!("Proposal not approved");
        }

        // Check if pool has enough funds
        if pool_status.total_invested < proposal.amount {
            log!(&env, "Insufficient pool funds");
            panic!("Insufficient pool funds");
        }

        // Mark proposal as executed
        proposal.is_executed = true;

        // Update pool status
        let mut updated_pool_status = pool_status;
        updated_pool_status.active_proposals -= 1;
        updated_pool_status.executed_proposals += 1;
        updated_pool_status.total_invested -= proposal.amount;

        // Store updated data
        env.storage()
            .instance()
            .set(&ProposalBook::Proposal(proposal_id), &proposal);
        env.storage()
            .instance()
            .set(&POOL_STATUS, &updated_pool_status);
        env.storage().instance().extend_ttl(5000, 5000);

        log!(
            &env,
            "Proposal executed! ID: {}, Amount: {}",
            proposal_id,
            proposal.amount
        );
    }

    // View function: Get pool status
    pub fn view_pool_status(env: Env) -> PoolStatus {
        env.storage()
            .instance()
            .get(&POOL_STATUS)
            .unwrap_or(PoolStatus {
                total_invested: 0,
                total_investors: 0,
                active_proposals: 0,
                executed_proposals: 0,
            })
    }

    // View function: Get investor details
    pub fn view_investor(env: Env, investor: Address) -> Investor {
        env.storage()
            .instance()
            .get(&InvestorBook::Investor(investor.clone()))
            .unwrap_or(Investor {
                address: investor,
                invested_amount: 0,
                voting_power: 0,
                joined_at: 0,
            })
    }

    // View function: Get proposal details
    pub fn view_proposal(env: Env, proposal_id: u64) -> Proposal {
        env.storage()
            .instance()
            .get(&ProposalBook::Proposal(proposal_id))
            .unwrap_or(Proposal {
                proposal_id: 0,
                title: String::from_str(&env, "Not_Found"),
                description: String::from_str(&env, "Not_Found"),
                amount: 0,
                votes_for: 0,
                votes_against: 0,
                is_executed: false,
                created_at: 0,
            })
    }
}
