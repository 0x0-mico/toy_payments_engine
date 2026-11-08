use std::process::ExitCode;

use clap::Parser;
use csv::Writer;
use thiserror::Error;

use crate::{
    engine::Engine,
    storage::Storage,
    transaction::{Event, RawEvent},
};

mod client;
mod engine;
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

    for result in reader.deserialize() {
        let raw: RawEvent = result?;
        let event: Event = raw.try_into();
        match event {
            Ok(ev) => {
                if let Err(err) = Engine::apply_event(raw.into(), &mut storage) {
                    eprintln!("{err}");
                }
            }
        }
    }

    let mut writer = Writer::from_writer(std::io::stdout());
    for client in storage.get_accounts().values() {
        writer.serialize(client)?;
    }
    writer.flush()?;

    Ok(())
}
