use crate::Rowable;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ExchangeInfo {
    pub auto_delete: bool,
    pub durable: bool,
    pub internal: bool,
    pub name: String,
    #[serde(alias = "type")]
    pub t: String,
    pub user_who_performed_action: String,
    pub vhost: String,
}

impl ExchangeInfo {
    pub fn headers<'a>() -> [&'a str; 2] {
        ["Name", "Type"]
    }
}

impl Rowable for ExchangeInfo {
    fn to_row(&self) -> Vec<String> {
        let nice_name = if self.name.is_empty() {
            "(AMQP DEFAULT)".to_owned()
        } else {
            self.name.clone()
        };

        vec![nice_name, self.t.clone()]
    }
}

#[derive(Deserialize, Debug)]
pub struct ExchangeBindings {
    pub source: String,
    pub vhost: String,
    #[serde(alias = "destination")]
    pub dest: String,
    #[serde(alias = "destination_type")]
    pub dest_type: String,
    pub routing_key: String,
    #[serde(alias = "properties_key")]
    pub prop_key: String,
}

impl ExchangeBindings {
    pub fn headers<'a>() -> [&'a str; 2] {
        ["To", "Routing key"]
    }
}

impl Rowable for ExchangeBindings {
    fn to_row(&self) -> Vec<String> {
        vec![self.dest.clone(), self.routing_key.clone()]
    }
}

#[derive(Deserialize, Debug)]
pub struct Overview {
    pub queue_totals: OverviewQueueTotals,
    pub message_stats: OverviewMessageRates,
}

#[derive(Deserialize, Debug)]
pub struct OverviewQueueTotals {
    pub messages: f64,
    pub messages_ready: f64,
    #[serde(alias = "messages_unacknowledged")]
    pub messages_unacked: f64,
}

#[derive(Deserialize, Debug)]
pub struct OverviewMessageRates {
    pub disk_reads: f64,
    pub disk_reads_details: RateContainer,
    pub disk_writes: f64,
    pub disk_writes_details: RateContainer,
}

#[derive(Deserialize, Debug)]
pub struct RateContainer {
    pub rate: f64,
}

#[derive(Deserialize, Debug)]
pub struct QueueInfo {
    pub name: String,
    #[serde(alias = "type")]
    pub t: String,
    pub state: String,
    #[serde(alias = "messages_ready")]
    pub ready: u64,
    #[serde(alias = "messages_unacknowledged")]
    pub unacked: u64,
    #[serde(alias = "messages")]
    pub total: u64,
}

impl QueueInfo {
    pub fn headers<'a>() -> [&'a str; 6] {
        [
            "Name",
            "Type",
            "State",
            "Ready",
            "Unacked",
            "Total",
        ]
    }
}

impl Rowable for QueueInfo {
    fn to_row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.t.clone(),
            self.state.clone(),
            self.ready.to_string(),
            self.unacked.to_string(),
            self.total.to_string(),
        ]
    }
}
