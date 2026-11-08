use std::process::ExitCode;

use clap::Parser;
use thiserror::Error;

use crate::{
    client::{AccountingError, ClientRow},
    engine::Engine,
    event::RawEvent,
    storage::Storage,
};

mod client;
mod engine;
mod event;
mod storage;
mod transaction;

#[derive(Parser, Debug)]
struct Arguments {
    csv_file_path: String,
}

#[derive(Debug, Error)]
enum RunError {
    #[error("CSV reading error: {0}")]
    Csv(#[from] csv::Error),
    #[error("IO Error {0}")]
    Io(#[from] std::io::Error),
    #[error("Accounting error: {0}")]
    Accounting(#[from] AccountingError),
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Fatal error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), RunError> {
    let args = Arguments::parse();

    let mut storage = Storage::new();
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(args.csv_file_path)?;

    let mut rows = reader.deserialize::<RawEvent>();
    let mut line = rows.reader().position().line();
    while let Some(result) = rows.next() {
        let row_line = line;
        line = rows.reader().position().line();
        let raw: RawEvent = match result {
            Ok(raw) => raw,
            Err(e) => {
                eprintln!("LN {row_line}: Error parsing event: {e}");
                continue;
            }
        };
        let event = raw.try_into();
        match event {
            Ok(ev) => {
                if let Err(err) = Engine::apply_event(ev, &mut storage) {
                    eprintln!("LN {row_line}: Error applying event: {err}");
                }
            }
            Err(e) => {
                eprintln!("LN {row_line}: Error parsing event: {e}")
            }
        }
    }

    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(std::io::stdout());
    writer.write_record(["client", "available", "held", "total", "locked"])?;
    for (client_id, account) in storage.sorted_clients() {
        writer.serialize(ClientRow::new(client_id, account)?)?;
    }
    writer.flush()?;

    Ok(())
}
