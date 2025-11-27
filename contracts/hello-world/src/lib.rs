#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Env, String, Symbol,
};

const PAY_NS: Symbol = symbol_short!("PAYRCPT"); // key namespace

#[contracttype]
#[derive(Clone)]
pub struct PayReceipt {
    pub receipt_id: u64,       // unique receipt id
    pub payer: Symbol,         // who paid
    pub payee: Symbol,         // who received
    pub currency: String,      // e.g., "USDC", "INR_OFFCHAIN"
    pub amount: i128,          // use i128 for amounts, typical pattern in tokens
    pub reference: String,     // invoice / payroll / tx ref
    pub paid_at: u64,          // ledger timestamp
    pub revoked: bool,         // if false = valid proof-of-pay
}

#[contract]
pub struct ProofOfPayToken;

#[contractimpl]
impl ProofOfPayToken {
    /// Issue a proof-of-payment receipt (intended to be called by payroll/payment system).
    pub fn issue_receipt(
        env: Env,
        receipt_id: u64,
        payer: Symbol,
        payee: Symbol,
        currency: String,
        amount: i128,
        reference: String,
    ) {
        let key = Self::receipt_key(receipt_id);
        // Prevent accidental overwrite
        let existing: Option<PayReceipt> = env.storage().instance().get(&key);
        if existing.is_some() {
            panic!("Receipt id already exists");
        }

        let paid_at = env.ledger().timestamp();
        let receipt = PayReceipt {
            receipt_id,
            payer,
            payee,
            currency,
            amount,
            reference,
            paid_at,
            revoked: false,
        };

        env.storage().instance().set(&key, &receipt);
    }

    /// Revoke a previously issued receipt (e.g., refund or error).
    pub fn revoke_receipt(env: Env, receipt_id: u64) {
        let key = Self::receipt_key(receipt_id);
        let mut receipt: PayReceipt = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or_else(|| panic!("Receipt not found"));

        receipt.revoked = true;
        env.storage().instance().set(&key, &receipt);
    }

    /// Check if a given receipt is a valid (non-revoked) proof of payment.
    pub fn is_receipt_valid(env: Env, receipt_id: u64) -> bool {
        let key = Self::receipt_key(receipt_id);
        let rec: Option<PayReceipt> = env.storage().instance().get(&key);
        match rec {
            Some(r) => !r.revoked,
            None => false,
        }
    }

    /// Get full receipt details for audits, HR, or compliance checks.
    pub fn get_receipt(env: Env, receipt_id: u64) -> Option<PayReceipt> {
        let key = Self::receipt_key(receipt_id);
        env.storage().instance().get(&key)
    }

    /// Internal helper: composite storage key under PAY_NS.
    fn receipt_key(receipt_id: u64) -> (Symbol, u64) {
        (PAY_NS, receipt_id)
    }
}
