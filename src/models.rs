use crate::{client::Ackmode, Rowable};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

trait ToRate {
    fn to_rate(&self) -> String;
}

impl ToRate for String {
    fn to_rate(&self) -> String {
        format!("{}/s", self)
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum MQEncoding {
    Auto,
}

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
    pub message_stats: QueueMsgStats,
    #[serde(alias = "messages_ready")]
    pub ready: u64,
    #[serde(alias = "messages_unacknowledged")]
    pub unacked: u64,
    #[serde(alias = "messages")]
    pub total: u64,
    pub vhost: String,
}

#[derive(Deserialize, Debug)]
pub struct QueueMsgStats {
    pub publish: u64,
    pub publish_details: RateContainer,
    pub deliver_get: u64,
    pub deliver_get_details: RateContainer,
    pub ack: u64,
    pub ack_details: RateContainer,
}

impl QueueInfo {
    pub fn headers<'a>() -> [&'a str; 9] {
        [
            "Name",
            "Type",
            "State",
            "Ready",
            "Unacked",
            "Total",
            "Incoming",
            "Deliver / Get",
            "Ack",
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
            self.message_stats.ack_details.rate.to_string().to_rate(),
            self.message_stats.deliver_get_details.rate.to_string().to_rate(),
            self.message_stats.ack_details.rate.to_string().to_rate(),
        ]
    }
}

#[derive(Serialize, Debug)]
pub struct PayloadPost {
    pub properties: HashMap<String, String>,
    pub routing_key: String,
    pub payload: String,
    #[serde(rename = "payload_encoding")]
    pub encoding: String,
}

impl Default for PayloadPost {
    fn default() -> Self {
        Self {
            properties: HashMap::new(),
            routing_key: "".to_string(),
            payload: "".to_string(),
            encoding: "string".to_string(),
        }
    }
}

impl PayloadPost {
    pub fn routing_key(mut self, key: String) -> Self {
        self.routing_key = key;
        self
    }

    pub fn payload(mut self, payload: String) -> Self {
        self.payload = payload;
        self
    }
}

#[derive(Serialize, Debug)]
pub struct MQMessageGetBody {
    count: u64,
    ackmode: Ackmode,
    encoding: MQEncoding,
}

impl Default for MQMessageGetBody {
    fn default() -> Self {
        Self {
            count: 1,
            ackmode: Ackmode::RejectRequeueTrue,
            encoding: MQEncoding::Auto,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct MQMessage {
    pub payload_bytes: u64,
    pub redelivered: bool,
    pub exchange: String,
    pub routing_key: String,
    pub payload: String,
}
