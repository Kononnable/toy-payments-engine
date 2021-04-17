use itertools::Itertools;
use std::collections::HashMap;
use toy_payments_engine::client::Client;
use toy_payments_engine::types::Transaction;

fn main() {
    println!("Hello, world!");

    let transaction_list: Vec<Transaction> = vec![];
    let mut clients: HashMap<u16, Client> = HashMap::new();

    for chunk in &transaction_list.into_iter().chunks(1000) {
        // stable sort, so transactions with same client id should still be sorted chronologically
        let transactions_by_client = chunk.sorted_by_key(|x| x.client).group_by(|x| x.client);
        // TODO: Change to par_iter
        for (client_id, transactions) in transactions_by_client.into_iter() {
            // TODO: move before for
            let client = clients.entry(client_id).or_insert_with(Default::default);

            client.process_transactions(transactions);
        }
    }
}
