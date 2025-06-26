use anyhow::Result;
use clap::Parser;
mod service;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

const RUNTIME_WORKER_MULTIPLIER: usize = 2;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Optional config file path
	#[arg(short, long)]
	config: Option<String>,
}

fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let _args = Args::parse();

	let num_cpus = num_cpus::get();
	let worker_threads = (num_cpus * RUNTIME_WORKER_MULTIPLIER).max(16);

	let mut builder = tokio::runtime::Builder::new_multi_thread();
	builder
		.disable_lifo_slot()
		.enable_all()
		.worker_threads(worker_threads)
		.build()
		.unwrap()
		.block_on(async { service::MovementIndexerV2::run().await })
}
