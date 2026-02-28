use std::{collections::HashMap, fs, sync::Mutex};

use ark_bn254::{Bn254, Fr};
use ark_circom::read_zkey;
use ark_groth16::ProvingKey;
use ark_relations::r1cs::ConstraintMatrices;

use crate::circuit::artifact_loader::ArtifactLoader;

pub struct FsArtifactLoader {
    path: String,
    cache: Mutex<HashMap<String, (ProvingKey<Bn254>, ConstraintMatrices<Fr>)>>,
}

impl FsArtifactLoader {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn load_artifacts(
        &self,
        circuit_name: &str,
    ) -> Result<(ProvingKey<Bn254>, ConstraintMatrices<Fr>), String> {
        let zkey_path = format!("{}/{}.zkey", self.path, circuit_name);
        let mut zkey_file = fs::File::open(&zkey_path)
            .map_err(|e| format!("Failed to open zkey file {}: {}", zkey_path, e))?;

        let (proving_key, matrices) =
            read_zkey(&mut zkey_file).map_err(|e| format!("Failed to read zkey: {}", e))?;

        Ok((proving_key, matrices))
    }
}

#[async_trait::async_trait]
impl ArtifactLoader for FsArtifactLoader {
    async fn load_proving_key(&self, circuit_name: &str) -> Result<ProvingKey<Bn254>, String> {
        let mut cache = self.cache.lock().map_err(|e| e.to_string())?;

        if let Some((pk, _)) = cache.get(circuit_name) {
            return Ok(pk.clone());
        }

        let (pk, matrices) = self.load_artifacts(circuit_name)?;

        cache.insert(circuit_name.to_string(), (pk.clone(), matrices));
        Ok(pk)
    }

    async fn load_matrices(&self, circuit_name: &str) -> Result<ConstraintMatrices<Fr>, String> {
        let mut cache = self.cache.lock().map_err(|e| e.to_string())?;

        if let Some((_, matrices)) = cache.get(circuit_name) {
            return Ok(matrices.clone());
        }

        let (pk, matrices) = self.load_artifacts(circuit_name)?;
        cache.insert(circuit_name.to_string(), (pk, matrices.clone()));
        Ok(matrices)
    }
}
