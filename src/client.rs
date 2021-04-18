use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::{
    errors::TransactionProcessingError,
    input_types::{Transaction, TransactionType},
};

#[derive(Clone, Debug, PartialEq)]
enum BalanceChangeEntryType {
    Deposit,
    Withdrawal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BalanceChangeEntryStatus {
    Valid,
    ActiveDispute,
    ChargedBack,
}
#[derive(Clone, Debug, PartialEq)]
struct BalanceChangeEntry {
    pub ty: BalanceChangeEntryType,
    pub amount: Decimal,
    pub status: BalanceChangeEntryStatus,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Client {
    balance_changes: HashMap<u32, BalanceChangeEntry>,
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

#[cfg(test)]
mod tests {
    use super::*;

    mod process_deposit {
        use super::*;

        #[test]
        fn should_increase_funds() {
            let mut client = Client::default();
            let amount = Decimal::new(1, 4);
            client
                .process_deposit(Transaction {
                    amount: Some(amount),
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Deposit,
                })
                .unwrap();
            assert_eq!(client.available, amount);
            assert_eq!(client.total(), amount);
            assert_eq!(client.balance_changes.len(), 1);
        }

        #[test]
        fn should_fail_on_reused_transaction_id() {
            let mut client = Client::default();
            let amount = Decimal::new(1, 0);
            client
                .process_deposit(Transaction {
                    amount: Some(amount),
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Deposit,
                })
                .unwrap();
            let original = client.clone();
            let result = client.process_deposit(Transaction {
                amount: Some(amount),
                client: 0,
                tx: 1,
                ty: TransactionType::Deposit,
            });

            assert_eq!(
                TransactionProcessingError::ReusedTransactionId,
                result.err().unwrap()
            );
            assert_eq!(original, client);
        }
    }
    mod process_withdrawal {
        use super::*;

        #[test]
        fn should_decrease_funds() {
            let mut client = Client {
                available: Decimal::new(1, 0),
                ..Default::default()
            };
            let amount = Decimal::new(1, 4);
            client
                .process_withdrawal(Transaction {
                    amount: Some(amount),
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Withdrawal,
                })
                .unwrap();
            let expected = Decimal::new(9999, 4);
            assert_eq!(client.available, expected);
            assert_eq!(client.total(), expected);
            assert_eq!(client.balance_changes.len(), 1);
        }

        #[test]
        fn should_fail_on_not_enough_funds() {
            let mut client = Client {
                available: Decimal::new(1, 0),
                ..Default::default()
            };
            let amount = Decimal::new(2, 0);
            let original = client.clone();
            let result = client.process_withdrawal(Transaction {
                amount: Some(amount),
                client: 0,
                tx: 1,
                ty: TransactionType::Withdrawal,
            });
            assert_eq!(
                TransactionProcessingError::NoSufficientFunds,
                result.err().unwrap()
            );
            assert_eq!(original, client);
        }
        #[test]
        fn should_fail_on_reused_transaction_id() {
            let mut client = Client {
                available: Decimal::new(10, 0),
                ..Default::default()
            };
            let amount = Decimal::new(1, 0);
            client
                .process_withdrawal(Transaction {
                    amount: Some(amount),
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Withdrawal,
                })
                .unwrap();
            let original = client.clone();
            let result = client.process_withdrawal(Transaction {
                amount: Some(amount),
                client: 0,
                tx: 1,
                ty: TransactionType::Withdrawal,
            });

            assert_eq!(
                TransactionProcessingError::ReusedTransactionId,
                result.err().unwrap()
            );
            assert_eq!(original, client);
        }
    }

    mod process_dispute {

        use super::*;

        fn create_test_client() -> Client {
            let mut client = Client::default();
            let amount = Decimal::new(1, 0);
            client
                .process_deposit(Transaction {
                    amount: Some(amount),
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Deposit,
                })
                .unwrap();
            client
        }
        #[test]
        fn should_block_funds() {
            let mut client = create_test_client();
            client
                .process_dispute(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Dispute,
                })
                .unwrap();
            assert_eq!(client.available, Decimal::new(0, 0));
            assert_eq!(client.held, Decimal::new(1, 0));
            assert_eq!(client.total(), Decimal::new(1, 0));
        }
        #[test]
        fn should_change_entry_status() {
            let mut client = create_test_client();
            client
                .process_dispute(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Dispute,
                })
                .unwrap();
            assert_eq!(client.balance_changes.len(), 1);
            assert_eq!(
                client.balance_changes.get(&1).unwrap().status,
                BalanceChangeEntryStatus::ActiveDispute
            );
        }
        #[test]
        fn should_fail_on_double_dispute() {
            let mut client = create_test_client();
            client
                .process_dispute(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Dispute,
                })
                .unwrap();
            let original = client.clone();
            let result = client.process_dispute(Transaction {
                amount: None,
                client: 0,
                tx: 1,
                ty: TransactionType::Dispute,
            });

            assert_eq!(
                TransactionProcessingError::DoubleDispute,
                result.err().unwrap()
            );
            assert_eq!(original, client);
        }
        #[test]
        fn should_fail_on_chargeback_transaction() {
            let mut client = create_test_client();
            client
                .process_dispute(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Dispute,
                })
                .unwrap();
            client
                .process_chargeback(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Chargeback,
                })
                .unwrap();
            let original = client.clone();
            let result = client.process_dispute(Transaction {
                amount: None,
                client: 0,
                tx: 1,
                ty: TransactionType::Dispute,
            });

            assert_eq!(
                TransactionProcessingError::DoubleDispute,
                result.err().unwrap()
            );
            assert_eq!(original, client);
        }
        #[test]
        fn should_fail_on_nonexisting_transaction() {
            let mut client = Client::default();
            let result = client.process_dispute(Transaction {
                amount: None,
                client: 0,
                tx: 1,
                ty: TransactionType::Dispute,
            });
            let original = client.clone();
            assert_eq!(
                TransactionProcessingError::UnknownTransactionId,
                result.err().unwrap()
            );
            assert_eq!(original, client);
        }
    }

    mod process_resolve {
        use super::*;

        fn create_test_client() -> Client {
            let mut client = Client::default();
            client
                .process_deposit(Transaction {
                    amount: Some(Decimal::new(1, 0)),
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Deposit,
                })
                .unwrap();
            client
                .process_dispute(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Dispute,
                })
                .unwrap();
            client
        }

        #[test]
        fn should_make_funds_available() {
            let mut client = create_test_client();
            client
                .process_resolve(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Resolve,
                })
                .unwrap();
            assert_eq!(client.available, Decimal::new(1, 0));
            assert_eq!(client.held, Decimal::new(0, 0));
            assert_eq!(client.total(), Decimal::new(1, 0));
        }
        #[test]
        fn should_change_entry_status() {
            let mut client = create_test_client();
            client
                .process_resolve(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Resolve,
                })
                .unwrap();
            assert_eq!(client.balance_changes.len(), 1);
            assert_eq!(
                client.balance_changes.get(&1).unwrap().status,
                BalanceChangeEntryStatus::Valid
            );
        }
        #[test]
        fn should_fail_on_valid_transaction() {
            let mut client = Client::default();
            client
                .process_deposit(Transaction {
                    amount: Some(Decimal::new(1, 0)),
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Deposit,
                })
                .unwrap();
            let original = client.clone();
            let result = client.process_resolve(Transaction {
                amount: None,
                client: 0,
                tx: 1,
                ty: TransactionType::Resolve,
            });
            assert_eq!(
                TransactionProcessingError::DisputeNotActive,
                result.err().unwrap()
            );
            assert_eq!(original, client);
        }
        #[test]
        fn should_fail_on_chargeback_transaction() {
            let mut client = create_test_client();
            client
                .process_chargeback(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Chargeback,
                })
                .unwrap();
            let original = client.clone();
            let result = client.process_resolve(Transaction {
                amount: None,
                client: 0,
                tx: 1,
                ty: TransactionType::Resolve,
            });

            assert_eq!(
                TransactionProcessingError::DisputeNotActive,
                result.err().unwrap()
            );
            assert_eq!(original, client);
        }
        #[test]
        fn should_fail_on_nonexisting_transaction() {
            let mut client = Client::default();
            let original = client.clone();
            let result = client.process_resolve(Transaction {
                amount: None,
                client: 0,
                tx: 1,
                ty: TransactionType::Resolve,
            });
            assert_eq!(
                TransactionProcessingError::UnknownTransactionId,
                result.err().unwrap()
            );
            assert_eq!(original, client);
        }
    }
    mod process_chargeback {
        use super::*;

        fn create_test_client() -> Client {
            let mut client = Client::default();
            client
                .process_deposit(Transaction {
                    amount: Some(Decimal::new(1, 0)),
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Deposit,
                })
                .unwrap();
            client
                .process_dispute(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Dispute,
                })
                .unwrap();
            client
        }
        #[test]
        fn should_reverse_transaction() {
            let mut client = create_test_client();
            client
                .process_chargeback(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Chargeback,
                })
                .unwrap();
            assert_eq!(client.available, Decimal::new(0, 0));
            assert_eq!(client.held, Decimal::new(0, 0));
            assert_eq!(client.total(), Decimal::new(0, 0));
        }
        #[test]
        fn should_change_entry_status() {
            let mut client = create_test_client();
            client
                .process_chargeback(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Chargeback,
                })
                .unwrap();
            assert_eq!(client.balance_changes.len(), 1);
            assert_eq!(
                client.balance_changes.get(&1).unwrap().status,
                BalanceChangeEntryStatus::ChargedBack
            );
        }
        #[test]
        fn should_freeze_account() {
            let mut client = create_test_client();
            client
                .process_chargeback(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Chargeback,
                })
                .unwrap();
            assert_eq!(client.is_frozen, true);
        }
        #[test]
        fn should_fail_on_valid_transaction() {
            let mut client = Client::default();
            client
                .process_deposit(Transaction {
                    amount: Some(Decimal::new(1, 0)),
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Deposit,
                })
                .unwrap();
            let original = client.clone();
            let result = client.process_chargeback(Transaction {
                amount: None,
                client: 0,
                tx: 1,
                ty: TransactionType::Chargeback,
            });

            assert_eq!(
                TransactionProcessingError::DisputeNotActive,
                result.err().unwrap()
            );
            assert_eq!(original, client);
        }
        #[test]
        fn should_fail_on_chargeback_transaction() {
            let mut client = create_test_client();
            client
                .process_chargeback(Transaction {
                    amount: None,
                    client: 0,
                    tx: 1,
                    ty: TransactionType::Chargeback,
                })
                .unwrap();
            let original = client.clone();
            let result = client.process_chargeback(Transaction {
                amount: None,
                client: 0,
                tx: 1,
                ty: TransactionType::Chargeback,
            });
            assert_eq!(
                TransactionProcessingError::DisputeNotActive,
                result.err().unwrap()
            );
            assert_eq!(original, client);
        }
        #[test]
        fn should_fail_on_nonexisting_transaction() {
            let mut client = Client::default();
            let original = client.clone();
            let result = client.process_chargeback(Transaction {
                amount: None,
                client: 0,
                tx: 1,
                ty: TransactionType::Chargeback,
            });
            assert_eq!(
                TransactionProcessingError::UnknownTransactionId,
                result.err().unwrap()
            );
            assert_eq!(original, client);
        }
    }
}
