use crate::AppError;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    Retry {
        max_attempts: u32,
        backoff: Duration,
    },
    Restart,
    Escalate,
    Ignore,
}

#[derive(Debug)]
pub struct ErrorRecoveryManager {
    error_count: u32,
    last_error_time: Option<Instant>,
    recovery_strategy: RecoveryStrategy,
}

impl ErrorRecoveryManager {
    pub fn new(strategy: RecoveryStrategy) -> Self {
        Self {
            error_count: 0,
            last_error_time: None,
            recovery_strategy: strategy,
        }
    }

    pub async fn handle_error<F, T>(
        &mut self,
        error: AppError,
        recovery_fn: F,
    ) -> Result<T, AppError>
    where
        F: std::future::Future<Output = Result<T, AppError>>,
    {
        self.error_count += 1;
        self.last_error_time = Some(Instant::now());

        println!("🚨 Error #{}: {:?}", self.error_count, error);

        match &self.recovery_strategy {
            RecoveryStrategy::Retry {
                max_attempts,
                backoff,
            } => {
                if self.error_count <= *max_attempts {
                    println!(
                        "🔄 Retrying after error (attempt {}/{})",
                        self.error_count, max_attempts
                    );
                    tokio::time::sleep(*backoff).await;
                    recovery_fn.await
                } else {
                    println!("❌ Max retry attempts reached");
                    Err(AppError::Internal {
                        message: format!("Max retries exceeded: {}", error),
                    })
                }
            }
            RecoveryStrategy::Restart => {
                println!("🔄 Restarting after error");
                Err(AppError::Internal {
                    message: "Restart required".to_string(),
                })
            }
            RecoveryStrategy::Escalate => {
                println!("⬆️ Escalating error");
                Err(error)
            }
            RecoveryStrategy::Ignore => {
                println!("🤫 Ignoring error");
                recovery_fn.await
            }
        }
    }

    pub fn reset(&mut self) {
        self.error_count = 0;
        self.last_error_time = None;
    }
}
