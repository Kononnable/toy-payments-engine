use itertools::Itertools;
use std::collections::HashMap;
use toy_payments_engine::types::{
    BalanceChangeEntry, BalanceChangeEntryStatus, BalanceChangeEntryType, ClientList, Transaction,
    TransactionType,
};

fn main() {
    println!("Hello, world!");

    let transaction_list: Vec<Transaction> = vec![];
    let mut clients: ClientList = HashMap::new();

    for chunk in &transaction_list.into_iter().chunks(1000) {
        // stable sort, so transactions with same client id should still be sorted chronologically
        let transactions_by_client = chunk.sorted_by_key(|x| x.client).group_by(|x| x.client);
        // TODO: Change to par_iter
        for (client_id, transactions) in transactions_by_client.into_iter() {
            // TODO: move before for
            let client = clients.entry(client_id).or_insert_with(Default::default);

            for transaction in transactions {
                match transaction.ty {
                    TransactionType::Deposit => {
                        let mut balance_change = client.balance_changes.get_mut(&transaction.tx);
                        if balance_change.is_some() {
                            // partner error - transaction id used twice, ignoring
                            continue;
                        }
                        let amount = transaction.amount.unwrap_or_default(); // if empty partner error - no amount for deposit transaction
                        balance_change.replace(&mut BalanceChangeEntry {
                            amount,
                            status: BalanceChangeEntryStatus::Valid,
                            ty: BalanceChangeEntryType::Deposit,
                        });
                        client.available += amount;
                    }
                    TransactionType::Withdrawal => {
                        let mut balance_change = client.balance_changes.get_mut(&transaction.tx);
                        if balance_change.is_some() {
                            // partner error - transaction id used twice, ignoring
                            continue;
                        }
                        let amount = transaction.amount.unwrap_or_default(); // if empty partner error - no amount for deposit transaction

                        if client.available >= amount {
                            balance_change.replace(&mut BalanceChangeEntry {
                                amount,
                                status: BalanceChangeEntryStatus::Valid,
                                ty: BalanceChangeEntryType::Deposit,
                            });
                            client.available -= amount;
                        } else {
                            // no sufficient available funds
                        }
                    }
                    TransactionType::Dispute => {
                        let balance_change = client.balance_changes.get_mut(&transaction.tx);
                        if balance_change.is_none() {
                            // partner error - transaction doesn't exist
                            continue;
                        }
                        let mut balance_change = balance_change.unwrap();
                        match balance_change.status {
                            BalanceChangeEntryStatus::Valid => {
                                balance_change.status = BalanceChangeEntryStatus::ActiveDispute;
                                client.available -= balance_change.amount;
                                client.held += balance_change.amount
                            }
                            BalanceChangeEntryStatus::ActiveDispute
                            | BalanceChangeEntryStatus::ChargedBack => {
                                continue;
                                // partner error - multiple dispute on same transaction
                            }
                        }
                    }
                    TransactionType::Resolve => {
                        let balance_change = client.balance_changes.get_mut(&transaction.tx);
                        if balance_change.is_none() {
                            // partner error - transaction doesn't exist
                            continue;
                        }
                        let mut balance_change = balance_change.unwrap();
                        match balance_change.status {
                            BalanceChangeEntryStatus::ActiveDispute => {
                                balance_change.status = BalanceChangeEntryStatus::Valid;
                                client.available += balance_change.amount;
                                client.held -= balance_change.amount;
                            }
                            BalanceChangeEntryStatus::Valid
                            | BalanceChangeEntryStatus::ChargedBack => {
                                continue;
                                // partner error - resolve on transaction without active dispute
                            }
                        }
                    }
                    TransactionType::Chargeback => {
                        let balance_change = client.balance_changes.get_mut(&transaction.tx);
                        if balance_change.is_none() {
                            // partner error - transaction doesn't exist
                            continue;
                        }
                        let mut balance_change = balance_change.unwrap();
                        match balance_change.status {
                            BalanceChangeEntryStatus::Valid
                            | BalanceChangeEntryStatus::ChargedBack => {
                                // partner error - resolve on transaction without active dispute
                                continue;
                            }
                            BalanceChangeEntryStatus::ActiveDispute => {
                                client.is_frozen = true; // should also block next transactions?
                                client.held -= balance_change.amount;
                                balance_change.status = BalanceChangeEntryStatus::ChargedBack;
                            }
                        }
                    }
                }
            }
        }
    }
}
