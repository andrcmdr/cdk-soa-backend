use std::collections::HashMap;
use reqwest::multipart;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let base_url = "http://localhost:8080";

    // Read the configuration file
    let config_content = std::fs::read_to_string("./mempool_monitor.config.yaml")?;

    // Create a task
    let form = multipart::Form::new()
        .text("name", "sentient-testnet-monitor")
        .text("config_yaml", config_content)
        .text("db_schema", std::fs::read_to_string("./init_table.sql")?);

    let response = client
        .post(&format!("{}/api/tasks", base_url))
        .multipart(form)
        .send()
        .await?;

    if response.status().is_success() {
        let create_response: serde_json::Value = response.json().await?;
        println!("Task created: {}", serde_json::to_string_pretty(&create_response)?);

        let task_id = create_response["task_id"].as_str().unwrap();

        // List all tasks
        let response = client
            .get(&format!("{}/api/tasks", base_url))
            .send()
            .await?;

        let tasks: serde_json::Value = response.json().await?;
        println!("All tasks: {}", serde_json::to_string_pretty(&tasks)?);

        // Get specific task
        let response = client
            .get(&format!("{}/api/tasks/{}", base_url, task_id))
            .send()
            .await?;

        let task: serde_json::Value = response.json().await?;
        println!("Task details: {}", serde_json::to_string_pretty(&task)?);

        // Stop the task after 10 seconds
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        let response = client
            .post(&format!("{}/api/tasks/{}/stop", base_url, task_id))
            .send()
            .await?;

        let stop_response: serde_json::Value = response.json().await?;
        println!("Stop response: {}", serde_json::to_string_pretty(&stop_response)?);

    } else {
        println!("Error creating task: {}", response.text().await?);
    }

    Ok(())
}
