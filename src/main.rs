use payment_engine::{Engine, Transaction};

fn main() -> std::io::Result<()> {
    let txs_csv_file_path = std::env::args()
        .nth(1)
        .expect("Please provide the path to the transactions CSV file as the first argument.");

    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(&txs_csv_file_path)?;

    let mut engine = Engine::default();

    for result in reader.deserialize() {
        let tx: Transaction = result?;
        engine.process_transaction(tx);
    }

    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    wtr.write_record(&["client", "available", "held", "total", "locked"])?;

    for (client_id, account) in engine.accounts() {
        wtr.write_record(&[
            client_id.to_string(),
            account.available_funds().reduce().to_string(),
            account.held_funds().reduce().to_string(),
            account.total_funds().reduce().to_string(),
            account.is_locked().to_string(),
        ])?;
    }

    Ok(())
}
