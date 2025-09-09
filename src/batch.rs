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
        Self { artifact_address: Vec::new(batch_size), usage: Vec::new(batch_size), timestamp: Vec::new(batch_size) }
    }
}

impl BatchRevenueReport {
    pub fn new(batch_size: i32) -> Self {
        Self { artifact_address: Vec::new(batch_size), revenue: Vec::new(batch_size), timestamp: Vec::new(batch_size) }
    }
}

pub fn get_batch_usage_report( batch_size: i32) -> BatchUsageReport {
    let rows = db.get_unsubmitted_usage_reports(batch_size).await?;
    let mut usage_reports_batch = BatchUsageReport::new(batch_size);
    for row in rows {
        usage_reports_batch.artifact_address.push(row.get(0));
        usage_reports_batch.usage.push(row.get(1));
        usage_reports_batch.timestamp.push(row.get(2));
    }
    usage_reports_batch
}

pub fn get_batch_revenue_report(batch_size: i32) -> BatchRevenueReport {
    let rows = db.get_unsubmitted_revenue_reports(batch_size).await?;
    let mut revenue_reports_batch = BatchRevenueReport::new(batch_size);
    for row in rows {
        revenue_reports_batch.artifact_address.push(row.get(0));
        revenue_reports_batch.revenue.push(row.get(1));
        revenue_reports_batch.timestamp.push(row.get(2));
    }
    revenue_reports_batch
}