use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::{
    errors::TransactionProcessingError,
    types::{Transaction, TransactionType},
};

#[derive(Debug)]
enum BalanceChangeEntryType {
    Deposit,
    Withdrawal,
}

#[derive(Debug, PartialEq, Eq)]
enum BalanceChangeEntryStatus {
    Valid,
    ActiveDispute,
    ChargedBack,
}
#[derive(Debug)]
struct BalanceChangeEntry {
    pub ty: BalanceChangeEntryType,
    pub amount: Decimal,
    pub status: BalanceChangeEntryStatus,
}

#[derive(Debug, Default)]
pub struct Client {
    balance_changes: HashMap<u32, BalanceChangeEntry>,
    // TODO: Can be less then zero? Deposit -> withdraw -> dispute
    pub available: Decimal,
    pub held: Decimal,
    pub is_frozen: bool,
}

impl Client {
    pub fn total(&self) -> Decimal {
        self.available + self.held
    }
    pub fn process_transaction(&mut self, transaction: Transaction) {
        let result = match transaction.ty {
            TransactionType::Deposit => self.process_deposit(transaction),
            TransactionType::Withdrawal => self.process_withdrawal(transaction),
            TransactionType::Dispute => self.process_dispute(transaction),
            TransactionType::Resolve => self.process_resolve(transaction),
            TransactionType::Chargeback => self.process_chargeback(transaction),
        };
        if let Err(_err) = result {
            // ignoring partner/client errors
        }
    }

    fn process_deposit(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), TransactionProcessingError> {
        self.validate_transaction_uniqueness(&transaction)?;
        let amount = get_transaction_amount(&transaction)?;
        self.balance_changes.insert(
            transaction.tx,
            BalanceChangeEntry {
                amount,
                status: BalanceChangeEntryStatus::Valid,
                ty: BalanceChangeEntryType::Deposit,
            },
        );
        self.available += amount;
        Ok(())
    }

    fn process_withdrawal(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), TransactionProcessingError> {
        self.validate_transaction_uniqueness(&transaction)?;
        let amount = get_transaction_amount(&transaction)?;
        if self.available < amount {
            return Err(TransactionProcessingError::NoSufficientFunds);
        }
        self.balance_changes.insert(
            transaction.tx,
            BalanceChangeEntry {
                amount,
                status: BalanceChangeEntryStatus::Valid,
                ty: BalanceChangeEntryType::Withdrawal,
            },
        );
        self.available -= amount;
        Ok(())
    }

    fn process_dispute(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), TransactionProcessingError> {
        let mut balance_change = self.get_balance_change_entry(transaction.tx)?;
        if balance_change.status != BalanceChangeEntryStatus::Valid {
            return Err(TransactionProcessingError::DoubleDispute);
        }
        balance_change.status = BalanceChangeEntryStatus::ActiveDispute;
        let amount = balance_change.amount;
        self.available -= amount;
        self.held += amount;
        Ok(())
    }

    fn process_resolve(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), TransactionProcessingError> {
        let mut balance_change = self.get_balance_change_entry(transaction.tx)?;
        if balance_change.status != BalanceChangeEntryStatus::ActiveDispute {
            return Err(TransactionProcessingError::DisputeNotActive);
        }
        balance_change.status = BalanceChangeEntryStatus::Valid;
        let amount = balance_change.amount;
        self.available += amount;
        self.held -= amount;
        Ok(())
    }

    fn process_chargeback(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), TransactionProcessingError> {
        let mut balance_change = self.get_balance_change_entry(transaction.tx)?;
        if balance_change.status != BalanceChangeEntryStatus::ActiveDispute {
            return Err(TransactionProcessingError::DisputeNotActive);
        }
        balance_change.status = BalanceChangeEntryStatus::ChargedBack;
        let amount = balance_change.amount;
        self.held -= amount;
        self.is_frozen = true;
        Ok(())
    }

    fn validate_transaction_uniqueness(
        &self,
        transaction: &Transaction,
    ) -> Result<(), TransactionProcessingError> {
        if self.balance_changes.contains_key(&transaction.tx) {
            return Err(TransactionProcessingError::ReusedTransactionId);
        }
        Ok(())
    }

    fn get_balance_change_entry(
        &mut self,
        tx: u32,
    ) -> Result<&mut BalanceChangeEntry, TransactionProcessingError> {
        let balance_change = self
            .balance_changes
            .get_mut(&tx)
            .ok_or(TransactionProcessingError::UnknownTransactionId)?;
        Ok(balance_change)
    }
}

fn get_transaction_amount(
    transaction: &Transaction,
) -> Result<Decimal, TransactionProcessingError> {
    transaction
        .amount
        .ok_or(TransactionProcessingError::AmountNotSpecified)
}
