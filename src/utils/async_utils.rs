use tokio::time::{Duration, sleep};

pub async fn simulate_network_delay() -> String {
    sleep(Duration::from_millis(1000));
    "Network response received".to_string()
}

pub async fn handle_multiple_requests() {
    let mut handles = vec![];
    for i in 1..=3 {
        let handle = tokio::spawn(async move {
            let result = simulate_network_delay().await;
            println!("Task {}: {}", i, result);
            result
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await.unwrap();
    }
}
