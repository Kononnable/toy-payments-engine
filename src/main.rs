use csv::ReaderBuilder;
use std::io::Write;
use std::{collections::HashMap, env};
use toy_payments_engine::client::Client;
use toy_payments_engine::types::Transaction;

fn main() {
    let path: String = env::args().nth(1).unwrap();

    let csv_reader = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)
        .unwrap();

    let mut clients: HashMap<u16, Client> = HashMap::new();

    for transaction in csv_reader
        .into_deserialize()
        .filter_map(|x: Result<Transaction, _>| x.ok())
    {
        let client = clients
            .entry(transaction.client)
            .or_insert_with(Default::default);

        client.process_transaction(transaction);
    }

    let stdout = std::io::stdout();
    let lock = stdout.lock();
    let mut writer = std::io::BufWriter::new(lock);

    writeln!(&mut writer, "client,available,held,total,locked").unwrap();
    for (id, client) in clients {
        writeln!(
            &mut writer,
            "{},{},{},{},{}",
            id,
            client.available,
            client.held,
            client.total(),
            client.is_frozen
        )
        .unwrap();
    }
}
