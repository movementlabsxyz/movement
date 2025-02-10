use aptos_api::{runtime::Apis, Context};
use tokio::try_join;

use std::sync::Arc;

pub struct Services {
	opt: maptos_opt_executor::Service,
	fin: maptos_fin_view::Service,
}

impl Services {
	pub(crate) fn new(opt: maptos_opt_executor::Service, fin: maptos_fin_view::Service) -> Self {
		Services { opt, fin }
	}

	pub fn opt_api_context(&self) -> Arc<Context> {
		self.opt.api_context()
	}

	pub fn get_opt_apis(&self) -> Apis {
		self.opt.get_apis()
	}

	pub fn get_fin_apis(&self) -> Apis {
		self.fin.get_apis()
	}

	pub async fn run(self) -> anyhow::Result<()> {
		let (opt_res, fin_res) =
			try_join!(tokio::spawn(self.opt.run()), tokio::spawn(self.fin.run()))?;
		opt_res.and(fin_res)
	}
}
