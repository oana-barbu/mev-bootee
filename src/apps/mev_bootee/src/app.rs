use std::prelude::v1::*;

use apps::AppEnv;
use base::trace::Alive;

#[derive(Default)]
pub struct MevBooTEE {
    pub alive: Alive,
}

impl apps::App for MevBooTEE {
    fn run(&self, args: AppEnv) -> Result<(), String> {
        glog::info!("running app");
        Ok(())
    }

    fn terminate(&self) {
        glog::info!("terminate MevBooTEE");
        self.alive.shutdown();
    }
}
