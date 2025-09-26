use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, oneshot};
use tokio::task::JoinHandle;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use tracing::{info, error, warn};

use crate::subscriptions::EventProcessor;
use crate::config::AppCfg;
use crate::{db, nats};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed(String),
}

pub struct Task {
    pub info: TaskInfo,
    pub handle: JoinHandle<anyhow::Result<()>>,
    pub shutdown_sender: Option<oneshot::Sender<()>>,
}

pub struct TaskManager {
    tasks: Arc<RwLock<HashMap<String, Task>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_task(
        &self,
        name: String,
        config: AppCfg,
        db_schema: String,
    ) -> anyhow::Result<String> {
        let task_id = Uuid::new_v4().to_string();

        info!("Creating new task: {} ({})", name, task_id);

        let task_info = TaskInfo {
            id: task_id.clone(),
            name: name.clone(),
            status: TaskStatus::Starting,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Create shutdown channel
        let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();

        // Clone necessary data for the task
        let tasks_clone = Arc::clone(&self.tasks);
        let task_id_clone = task_id.clone();

        // Spawn the task
        let handle = tokio::spawn(async move {
            // Update status to starting
            {
                let mut tasks = tasks_clone.write().await;
                if let Some(task) = tasks.get_mut(&task_id_clone) {
                    task.info.status = TaskStatus::Starting;
                    task.info.updated_at = chrono::Utc::now();
                }
            }

            // Initialize database connection
            let pg = match db::connect_pg(&config.postgres.dsn, &db_schema).await {
                Ok(client) => client,
                Err(e) => {
                    error!("Failed to connect to database for task {}: {:?}", task_id_clone, e);

                    // Update status to failed
                    let mut tasks = tasks_clone.write().await;
                    if let Some(task) = tasks.get_mut(&task_id_clone) {
                        task.info.status = TaskStatus::Failed(format!("Database connection failed: {}", e));
                        task.info.updated_at = chrono::Utc::now();
                    }
                    return Err(e);
                }
            };

            // Initialize NATS if enabled
            let nats = if config.nats.nats_enabled.is_some_and(|enabled| enabled > 0) {
                match nats::connect(&config.nats.url, &config.nats.object_store_bucket).await {
                    Ok(nats_client) => Some(nats_client),
                    Err(e) => {
                        warn!("Failed to connect to NATS for task {}: {:?}", task_id_clone, e);
                        None
                    }
                }
            } else {
                None
            };

            // Create event processor
            let event_processor = match EventProcessor::new(&config, pg, nats).await {
                Ok(processor) => processor,
                Err(e) => {
                    error!("Failed to create EventProcessor for task {}: {:?}", task_id_clone, e);

                    // Update status to failed
                    let mut tasks = tasks_clone.write().await;
                    if let Some(task) = tasks.get_mut(&task_id_clone) {
                        task.info.status = TaskStatus::Failed(format!("EventProcessor creation failed: {}", e));
                        task.info.updated_at = chrono::Utc::now();
                    }
                    return Err(e);
                }
            };

            // Update status to running
            {
                let mut tasks = tasks_clone.write().await;
                if let Some(task) = tasks.get_mut(&task_id_clone) {
                    task.info.status = TaskStatus::Running;
                    task.info.updated_at = chrono::Utc::now();
                }
            }

            info!("Task {} ({}) is now running", name, task_id_clone);

            // Run the event processor with shutdown handling
            let processor_result = tokio::select! {
                result = event_processor.run() => {
                    info!("Task {} completed: {:?}", task_id_clone, result);
                    result
                }
                _ = shutdown_receiver => {
                    info!("Task {} received shutdown signal", task_id_clone);
                    Ok(())
                }
            };

            // Update final status
            {
                let mut tasks = tasks_clone.write().await;
                if let Some(task) = tasks.get_mut(&task_id_clone) {
                    task.info.status = match &processor_result {
                        Ok(_) => TaskStatus::Stopped,
                        Err(e) => TaskStatus::Failed(e.to_string()),
                    };
                    task.info.updated_at = chrono::Utc::now();
                }
            }

            processor_result
        });

        // Store the task
        let task = Task {
            info: task_info,
            handle,
            shutdown_sender: Some(shutdown_sender),
        };

        let mut tasks = self.tasks.write().await;
        tasks.insert(task_id.clone(), task);

        info!("Task {} created successfully", task_id);
        Ok(task_id)
    }

    pub async fn stop_task(&self, task_id: &str) -> anyhow::Result<()> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            info!("Stopping task: {} ({})", task.info.name, task_id);

            // Update status to stopping
            task.info.status = TaskStatus::Stopping;
            task.info.updated_at = chrono::Utc::now();

            // Send shutdown signal if sender is available
            if let Some(sender) = task.shutdown_sender.take() {
                if let Err(_) = sender.send(()) {
                    warn!("Failed to send shutdown signal to task {}", task_id);
                }
            }

            Ok(())
        } else {
            Err(anyhow::anyhow!("Task not found: {}", task_id))
        }
    }

    pub async fn get_task(&self, task_id: &str) -> Option<TaskInfo> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).map(|task| task.info.clone())
    }

    pub async fn list_tasks(&self) -> Vec<TaskInfo> {
        let tasks = self.tasks.read().await;
        tasks.values().map(|task| task.info.clone()).collect()
    }

    pub async fn cleanup_finished_tasks(&self) {
        let mut tasks = self.tasks.write().await;
        let mut to_remove = Vec::new();

        for (id, task) in tasks.iter() {
            if task.handle.is_finished() {
                match &task.info.status {
                    TaskStatus::Stopped | TaskStatus::Failed(_) => {
                        to_remove.push(id.clone());
                    }
                    _ => {}
                }
            }
        }

        for id in to_remove {
            info!("Cleaning up finished task: {}", id);
            tasks.remove(&id);
        }
    }
}
