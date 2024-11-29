use anyhow::anyhow;
use clap::{Parser, Subcommand};
use core_affinity::{self as core, CoreId};
use crossbeam::channel;
use std::thread;

mod hash;
mod tui;

static MAX_MESSAGE_LENGTH: usize = 64;

#[derive(Subcommand, Debug)]
enum Command {
    Bench,
    Run,
}

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[arg(long)]
    username: String,

    #[arg(long)]
    message: Option<String>,

    #[arg(long)]
    iterations: Option<u64>,
}

impl Args {
    fn validate(&self) -> anyhow::Result<()> {
        if let Some(ref m) = self.message {
            if m.len() > MAX_MESSAGE_LENGTH {
                return Err(anyhow!(
                    "message cannot be more than {MAX_MESSAGE_LENGTH} characters"
                ));
            }
        }

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    args.validate()?;

    match args.command {
        Command::Bench => bench(args),
        Command::Run => run(args),
    }
}

fn bench(args: Args) -> anyhow::Result<()> {
    let prefix = hash::concat(args.username.clone(), args.message.clone());
    let (hash, nonce) = hash::worker(
        CoreId { id: 0 },
        1,
        args.username,
        args.message,
        args.iterations,
        None,
    );

    println!("minimum = {}/{} = {}", prefix, nonce, hex::encode(hash));
    Ok(())
}

fn run(args: Args) -> anyhow::Result<()> {
    let prefix = hash::concat(args.username.clone(), args.message.clone());

    // worker thread reports minimum hash
    let (r, results) = channel::unbounded::<hash::Result>();

    // determine the number of cores (worker threads)
    let cores = core::get_core_ids().expect("get core ids");
    let n = cores.len();

    // spawn a worker for each core
    for core in cores.clone() {
        let username = args.username.clone();
        let message = args.message.clone();
        let iterations = args.iterations.clone();
        let results = r.clone();

        let _ = thread::spawn(move || {
            if !core::set_for_current(core) {
                eprintln!("could not set core affinity for {}", core.id);
            }

            hash::worker(core, n, username, message, iterations, Some(results));
        });
    }

    tui::run(tui::App::new(cores, results, prefix))?;
    Ok(())
}
