use std::{collections::HashMap, sync::Mutex};

use num_bigint::BigInt;
use ruint::aliases::U256;
use wasmer::Store;

use crate::circuit::witness::WitnessCalculator;

pub struct WasmerWitnessCalculator {
    path: String,
    inner: Mutex<Option<WitnessCalcState>>,
}

struct WitnessCalcState {
    store: Store,
    calculator: ark_circom::WitnessCalculator,
    circuit_name: String,
}

impl WasmerWitnessCalculator {
    pub fn new(wasm_path: &str) -> Self {
        Self {
            path: wasm_path.to_string(),
            inner: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl WitnessCalculator for WasmerWitnessCalculator {
    async fn calculate_witness(
        &self,
        circuit_name: &str,
        inputs: HashMap<String, Vec<U256>>,
    ) -> Result<Vec<U256>, String> {
        let wasm_path = format!("{}/{}.wasm", self.path, circuit_name);
        let mut guard = self.inner.lock().map_err(|e| e.to_string())?;

        // Check if we have a cached calculator for this circuit type
        let needs_reload = match &*guard {
            Some(state) => state.circuit_name != circuit_name,
            None => true,
        };

        if needs_reload {
            let mut store = Store::default();
            let calculator = ark_circom::WitnessCalculator::new(&mut store, &wasm_path)
                .map_err(|e| e.to_string())?;
            *guard = Some(WitnessCalcState {
                store,
                calculator,
                circuit_name: circuit_name.to_string(),
            });
        }

        let state = guard.as_mut().unwrap();

        // Convert inputs from U256 to BigInt
        let inputs: HashMap<String, Vec<BigInt>> = inputs
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().map(BigInt::from).collect()))
            .collect();

        // Calculate witness
        let witness = state
            .calculator
            .calculate_witness(&mut state.store, inputs, true)
            .map_err(|e| e.to_string())?;

        // Convert witness to U256
        let witness: Vec<U256> = witness.into_iter().map(U256::from).collect();

        Ok(witness)
    }
}
