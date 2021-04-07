use crate::models::{
    ExchangeBindings, ExchangeInfo, MQMessage, MQMessageGetBody, Overview, PayloadPost, QueueInfo,
};
use crate::ManagementClient;

use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Ackmode {
    AckRequeueTrue,
    AckRequeueFalse,
    RejectRequeueTrue,
    RejectRequeueFalse,
}

pub struct Client {
    addr: String,
    user: String,
    pass: Option<String>,
    client: reqwest::blocking::Client,
}

impl Client {
    pub fn new(addr: &str, user: &str, pass: Option<String>) -> Self {
        Self {
            addr: addr.to_string(),
            user: user.to_string(),
            pass,
            client: reqwest::blocking::Client::new(),
        }
    }

    // TODO change this to Result and cover api failures!!
    pub fn get<T>(&self, endpoint: &str) -> Result<T, reqwest::Error>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}{}", self.addr, endpoint);
        self.client
            .get(url)
            .basic_auth(&self.user, self.pass.as_ref())
            .send()?
            .json()
    }

    pub fn post<T, S>(&self, endpoint: &str, body: &S) -> Result<T, reqwest::Error>
    where
        T: DeserializeOwned,
        S: Serialize,
    {
        let url = format!("{}{}", self.addr, endpoint);
        self.client
            .post(url)
            .basic_auth(&self.user, self.pass.as_ref())
            .json(body)
            .send()?
            .json()
    }
}

impl ManagementClient for Client {
    fn get_exchange_overview(&self) -> Vec<ExchangeInfo> {
        self.get::<Vec<ExchangeInfo>>("/api/exchanges").unwrap()
    }

    fn get_exchange_bindings(&self, exch: &ExchangeInfo) -> Vec<ExchangeBindings> {
        let n = exch.vhost.replace("/", "%2F");
        let endpoint = format!("/api/exchanges/{}/{}/bindings/source", n, exch.name);
        self.get::<Vec<ExchangeBindings>>(&endpoint).unwrap()
    }

    fn get_overview(&self) -> Overview {
        self.get::<Overview>("/api/overview").unwrap()
    }

    fn get_queues_info(&self) -> Vec<QueueInfo> {
        self.get::<Vec<QueueInfo>>("/api/queues").unwrap()
    }

    fn post_queue_payload(&self, queue_name: String, vhost: &str, payload: String) {
        let vhost_encoded = vhost.replace("/", "%2F");
        let endpoint = format!("{}/api/exchanges/{}//publish", self.addr, vhost_encoded);
        let body = PayloadPost::default()
            .routing_key(queue_name)
            .payload(payload);
        // TODO consider failures
        let _ = self
            .client
            .post(endpoint)
            .basic_auth("guest", Some("guest"))
            .json(&body)
            .send();
    }

    fn pop_queue_item(&self, queue_name: &str, vhost: &str) -> Option<MQMessage> {
        let vhost_encoded = vhost.replace("/", "%2F");
        let endpoint = format!("/api/queues/{}/{}/get", vhost_encoded, queue_name);
        let body = MQMessageGetBody::default();
        let mut res = self
            .post::<Vec<MQMessage>, MQMessageGetBody>(&endpoint, &body)
            .unwrap();
        if res.is_empty() {
            None
        } else {
            Some(res.remove(0))
        }
    }

    fn ping(&self) -> Result<(), ()> {
        // TODO better ping?
        match self.get::<Overview>("/api/overview") {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}
