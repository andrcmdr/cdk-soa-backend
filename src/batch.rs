use anyhow::Result;
use crate::db::Database;
use tracing::info;
pub struct BatchUsageReport{
    pub artifact_address: Vec<String>,
    pub usage: Vec<i64>,
    pub timestamp: Vec<i64>,
}

pub struct BatchRevenueReport{
    pub artifact_address: Vec<String>,
    pub revenue: Vec<i64>,
    pub timestamp: Vec<i64>,
}

impl BatchUsageReport {
    pub fn new(batch_size: i32) -> Self {
        Self { 
            artifact_address: Vec::with_capacity(batch_size as usize), 
            usage: Vec::with_capacity(batch_size as usize), 
            timestamp: Vec::with_capacity(batch_size as usize) 
        }
    }
}

impl BatchRevenueReport {
    pub fn new(batch_size: i32) -> Self {
        Self { 
            artifact_address: Vec::with_capacity(batch_size as usize), 
            revenue: Vec::with_capacity(batch_size as usize), 
            timestamp: Vec::with_capacity(batch_size as usize) 
        }
    }
}

pub async fn get_batch_usage_report(db: &Database, batch_size: i32) -> Result<(BatchUsageReport, Vec<i32>)> {
    let rows = db.get_unsubmitted_usage_reports(batch_size).await?;
    let actual_count = rows.len();
    info!("Found {} unsubmitted usage reports", actual_count);

    let mut usage_reports_batch = BatchUsageReport::new(actual_count as i32);
    let mut ids = Vec::with_capacity(actual_count as usize);
    
    for row in rows {
        ids.push(row.get::<_, i32>("id"));
        usage_reports_batch.artifact_address.push(row.get::<_, String>("artifact_address"));
        usage_reports_batch.usage.push(row.get::<_, i64>("usage"));
        usage_reports_batch.timestamp.push(row.get::<_, i64>("timestamp"));
    }
    Ok((usage_reports_batch, ids))
}

pub async fn get_batch_revenue_report(db: &Database, batch_size: i32) -> Result<(BatchRevenueReport, Vec<i32>)> {
    let rows = db.get_unsubmitted_revenue_reports(batch_size).await?;
    let actual_count = rows.len();
    info!("Found {} unsubmitted revenue reports", actual_count);  

    let mut revenue_reports_batch = BatchRevenueReport::new(batch_size);
    let mut ids = Vec::with_capacity(batch_size as usize);
    
    for row in rows {
        ids.push(row.get::<_, i32>("id"));
        revenue_reports_batch.artifact_address.push(row.get::<_, String>("artifact_address"));
        revenue_reports_batch.revenue.push(row.get::<_, i64>("revenue"));
        revenue_reports_batch.timestamp.push(row.get::<_, i64>("timestamp"));
    }
    Ok((revenue_reports_batch, ids))
}