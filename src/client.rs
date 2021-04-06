use crate::ManagementClient;
use crate::models::{ExchangeInfo, ExchangeBindings, Overview};

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

    pub fn get<T>(&self, url: String) -> T where
        T: DeserializeOwned
    {
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
        let endpoint = format!("{}{}", self.addr, "/api/exchanges");
        self.get::<Vec<ExchangeInfo>>(endpoint)
    }

    fn get_exchange_bindings(&self, exch: &ExchangeInfo) -> Vec<ExchangeBindings> {
        let n = match &*exch.vhost {
            "/" => "%2F".to_string(),
            _ => exch.vhost.clone(),
        };
        let endpoint = format!("{}/api/exchanges/{}/{}/bindings/source", self.addr, n, exch.name);
        self.get::<Vec<ExchangeBindings>>(endpoint)
    }

    fn get_overview(&self) -> Overview {
        let endpoint = format!("{}/api/overview", self.addr);
        self.get::<Overview>(endpoint)
    }
}
