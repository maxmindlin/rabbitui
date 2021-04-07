use crate::models::{ExchangeBindings, ExchangeInfo, Overview, QueueInfo};
use crate::ManagementClient;

use serde::de::DeserializeOwned;

pub struct Client {
    addr: String,
    client: reqwest::blocking::Client,
}

impl Client {
    pub fn new(addr: String) -> Self {
        Self {
            addr,
            client: reqwest::blocking::Client::new(),
        }
    }

    // TODO change this to Result and cover api failures!!
    pub fn get<T>(&self, endpoint: &str) -> T
    where
        T: DeserializeOwned,
    {
        let url = format!("{}{}", self.addr, endpoint);
        self.client
            .get(url)
            .basic_auth("guest", Some("guest"))
            .send()
            .unwrap()
            .json()
            .unwrap()
    }
}

impl ManagementClient for Client {
    fn get_exchange_overview(&self) -> Vec<ExchangeInfo> {
        self.get::<Vec<ExchangeInfo>>("/api/exchanges")
    }

    fn get_exchange_bindings(&self, exch: &ExchangeInfo) -> Vec<ExchangeBindings> {
        let n = exch.vhost.replace("/", "%2F");
        let endpoint = format!(
            "/api/exchanges/{}/{}/bindings/source",
            n, exch.name
        );
        self.get::<Vec<ExchangeBindings>>(&endpoint)
    }

    fn get_overview(&self) -> Overview {
        self.get::<Overview>("/api/overview")
    }

    fn get_queues_info(&self) -> Vec<QueueInfo> {
        self.get::<Vec<QueueInfo>>("/api/queues")
    }
}
