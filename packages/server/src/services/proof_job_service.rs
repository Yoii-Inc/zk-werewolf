use crate::{
    models::game::{BatchKey, BatchRequest},
    state::AppState,
    utils::config::CONFIG,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::{mpsc, Mutex};

#[derive(Debug, Clone)]
pub struct ProofJob {
    pub room_id: String,
    pub batch_key: BatchKey,
    pub batch_request: BatchRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeJobStatus {
    pub state: String, // pending/running/completed/failed/timeout
    pub attempt_count: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofJobStatus {
    pub state: String, // pending/running/completed/failed/timeout
    pub batch_id: String,
    pub room_id: String,
    pub batch_key: BatchKey,
    pub job_node_status: HashMap<String, NodeJobStatus>,
    pub attempt_count: u32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProofJobStatus {
    fn new(job: &ProofJob) -> Self {
        let mut job_node_status = HashMap::new();
        for node_url in CONFIG.zk_mpc_node_urls() {
            job_node_status.insert(
                node_url,
                NodeJobStatus {
                    state: "pending".to_string(),
                    attempt_count: 0,
                    last_error: None,
                },
            );
        }

        Self {
            state: "pending".to_string(),
            batch_id: job.batch_request.batch_id.clone(),
            room_id: job.room_id.clone(),
            batch_key: job.batch_key.clone(),
            job_node_status,
            attempt_count: 0,
            last_error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

pub struct ProofJobService {
    sender: mpsc::Sender<ProofJob>,
    receiver: Mutex<Option<mpsc::Receiver<ProofJob>>>,
    statuses: Arc<Mutex<HashMap<String, ProofJobStatus>>>,
    worker_started: AtomicBool,
}

impl ProofJobService {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(256);
        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
            statuses: Arc::new(Mutex::new(HashMap::new())),
            worker_started: AtomicBool::new(false),
        }
    }

    pub async fn enqueue_job(&self, app_state: AppState, job: ProofJob) -> Result<(), String> {
        self.ensure_worker_started(app_state.clone()).await;

        let initial_status_for_broadcast = {
            let mut statuses = self.statuses.lock().await;
            if let Some(existing) = statuses.get(&job.batch_request.batch_id) {
                if matches!(existing.state.as_str(), "pending" | "running") {
                    return Err(format!(
                        "proof job {} is already {}",
                        existing.batch_id, existing.state
                    ));
                }
            }
            let initial_status = ProofJobStatus::new(&job);
            statuses.insert(job.batch_request.batch_id.clone(), initial_status.clone());
            initial_status
        };

        if let Err(e) = app_state
            .broadcast_proof_job_status(&job.room_id, &initial_status_for_broadcast)
            .await
        {
            tracing::warn!(
                "Failed to broadcast pending proof job status for room {}: {}",
                job.room_id,
                e
            );
        }

        self.sender.send(job).await.map_err(|e| e.to_string())
    }

    pub async fn get_status(&self, batch_id: &str) -> Option<ProofJobStatus> {
        let statuses = self.statuses.lock().await;
        statuses.get(batch_id).cloned()
    }

    async fn ensure_worker_started(&self, app_state: AppState) {
        if self.worker_started.swap(true, Ordering::SeqCst) {
            return;
        }

        let mut receiver_guard = self.receiver.lock().await;
        let Some(mut receiver) = receiver_guard.take() else {
            return;
        };
        drop(receiver_guard);

        let statuses = self.statuses.clone();

        tokio::spawn(async move {
            while let Some(job) = receiver.recv().await {
                process_job(statuses.clone(), app_state.clone(), job).await;
            }
        });
    }
}

async fn process_job(
    statuses: Arc<Mutex<HashMap<String, ProofJobStatus>>>,
    app_state: AppState,
    job: ProofJob,
) {
    let batch_id = job.batch_request.batch_id.clone();
    let room_id = job.room_id.clone();
    let mut running_status_for_broadcast = None;

    {
        let mut status_map = statuses.lock().await;
        if let Some(status) = status_map.get_mut(&batch_id) {
            status.state = "running".to_string();
            status.attempt_count += 1;
            status.updated_at = Utc::now();
            for node_status in status.job_node_status.values_mut() {
                node_status.state = "running".to_string();
                node_status.attempt_count += 1;
            }
            running_status_for_broadcast = Some(status.clone());
        }
    }

    if let Some(status) = running_status_for_broadcast {
        if let Err(e) = app_state
            .broadcast_proof_job_status(&room_id, &status)
            .await
        {
            tracing::warn!(
                "Failed to broadcast running proof job status for room {}: {}",
                room_id,
                e
            );
        }
    }

    let execution_result =
        crate::services::zk_proof::execute_batch_request(&job.batch_request).await;
    let mut execution_error = execution_result.as_ref().err().cloned();

    crate::services::zk_proof::store_precomputed_batch_result(batch_id.clone(), execution_result)
        .await;

    if execution_error.is_none() {
        let mut games = app_state.games.lock().await;
        if let Some(game) = games.get_mut(&room_id) {
            game.apply_proof_result_for_batch(&app_state, &job.batch_key, job.batch_request)
                .await;
        } else {
            execution_error = Some(format!(
                "Game not found while applying proof result: room_id={}",
                room_id
            ));
        }
    }

    let mut final_status_for_broadcast = None;
    {
        let mut status_map = statuses.lock().await;
        if let Some(status) = status_map.get_mut(&batch_id) {
            status.updated_at = Utc::now();
            status.last_error = execution_error.clone();

            let next_state = match &execution_error {
                None => "completed",
                Some(err) if err.to_ascii_lowercase().contains("timeout") => "timeout",
                Some(_) => "failed",
            };
            status.state = next_state.to_string();

            for node_status in status.job_node_status.values_mut() {
                node_status.state = next_state.to_string();
                node_status.last_error = execution_error.clone();
            }
            final_status_for_broadcast = Some(status.clone());
        }
    }

    if let Some(status) = final_status_for_broadcast {
        if let Err(e) = app_state
            .broadcast_proof_job_status(&room_id, &status)
            .await
        {
            tracing::warn!(
                "Failed to broadcast finalized proof job status for room {}: {}",
                room_id,
                e
            );
        }
    }
}
