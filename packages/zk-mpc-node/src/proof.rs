use crate::models::ProofRequest;
use crate::models::ProofStatus;
use crate::ProofOutput;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// TODO: Changeable to a more generic proof manager
pub struct ProofManager {
    proofs: Arc<RwLock<HashMap<String, ProofStatus>>>,
}

impl Default for ProofManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProofManager {
    pub fn new() -> Self {
        ProofManager {
            proofs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn register_proof_request(&self, request: ProofRequest) {
        let status = ProofStatus {
            state: "pending".to_string(),
            proof_id: request.proof_id.clone(),
            message: None,
            output: None,
        };
        self.proofs
            .write()
            .await
            .insert(request.proof_id.clone(), status);
    }

    pub async fn get_proof_status(&self, proof_id: &str) -> Option<ProofStatus> {
        self.proofs.read().await.get(proof_id).cloned()
    }
    pub async fn update_proof_status(&self, proof_id: &str, state: &str, message: Option<String>) {
        if let Some(status) = self.proofs.write().await.get_mut(proof_id) {
            status.state = state.to_string();
            status.message = message;
        }

        println!(
            "Proof status updated for ID {}: {:?}",
            proof_id,
            self.get_proof_status(proof_id).await
        );
    }

    pub async fn update_proof_status_with_output(
        &self,
        proof_id: &str,
        state: &str,
        message: Option<String>,
        output: Option<ProofOutput>,
    ) {
        if let Some(status) = self.proofs.write().await.get_mut(proof_id) {
            status.state = state.to_string();
            status.message = message;
            status.output = output;
        }

        println!(
            "Proof status updated for ID {}: {:?}",
            proof_id,
            self.get_proof_status(proof_id).await
        );
    }
}
