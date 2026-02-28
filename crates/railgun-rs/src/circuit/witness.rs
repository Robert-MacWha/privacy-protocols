use std::collections::HashMap;

use ruint::aliases::U256;

#[async_trait::async_trait]
pub trait WitnessCalculator {
    async fn calculate_witness(
        &self,
        circuit_name: &str,
        inputs: HashMap<String, Vec<U256>>,
    ) -> Result<Vec<U256>, String>;
}
