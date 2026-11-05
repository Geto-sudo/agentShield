use std::sync::Arc;

use crate::config::Config;
use crate::engine::expert::ExpertService;
use crate::paypal::PayPalGateway;
use crate::store::Db;

/// État partagé par tous les handlers axum.
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<Config>,
    pub expert: Arc<ExpertService>,
    pub paypal: Arc<dyn PayPalGateway>,
}

impl AppState {
    pub fn new(
        db: Db,
        config: Arc<Config>,
        expert: Arc<ExpertService>,
        paypal: Arc<dyn PayPalGateway>,
    ) -> Self {
        Self {
            db,
            config,
            expert,
            paypal,
        }
    }
}
