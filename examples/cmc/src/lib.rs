use anyhow::Result;
use blocksense_sdk::{
    oracle::{DataFeedResult, Payload, Settings},
    oracle_component,
    spin::http::{send, Method, Request, Response},
};
use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;
use url::Url;

use std::fs;

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub status: Status,
    pub data: HashMap<u64, CmcData>,
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub timestamp: String,
    #[serde(rename = "error_code")]
    pub error_code: i64,
    #[serde(rename = "error_message")]
    pub error_message: Value,
    pub elapsed: i64,
    #[serde(rename = "credit_count")]
    pub credit_count: i64,
    pub notice: Value,
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CmcData {
    id: i64,
    quote: HashMap<String, CmcValue>,
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub slug: String,
    pub name: String,
    pub category: String,
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CmcValue {
    pub price: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct CmcResource {
    pub cmc_id: u64,
    pub cmc_quote: String,
}

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

#[oracle_component]
async fn oracle_request(settings: Settings) -> Result<Payload> {
    let mut resources: HashMap<String, CmcResource> = HashMap::new();
    let mut ids: Vec<String> = vec![];
    for feed in settings.data_feeds.iter() {
        let data: CmcResource = serde_json::from_str(&feed.data)?;
        resources.insert(feed.id.clone(), data.clone());
        ids.push(data.cmc_id.to_string());
    }

    let url = Url::parse_with_params(
        "https://pro-api.coinmarketcap.com/v2/cryptocurrency/quotes/latest",
        &[("id", ids.join(","))],
    )?;

    let mut req = Request::builder();
    req.method(Method::Get);
    req.uri(url);

    // Please provide your own API key until capabilities are implemented.
    let mut private_key: String = fs::read_to_string("/CMC_API_KEY")?;
    trim_newline(&mut private_key);
    // println!("Using private key for CoinMarketCap `{}`", &private_key);
    req.header("X-CMC_PRO_API_KEY", &private_key);
    req.header("Accepts", "application/json");

    let req = req.build();
    let resp: Response = send(req).await?;

    let body = resp.into_body();
    let string = String::from_utf8(body)?;
    let value: Root = serde_json::from_str(&string)?;
    let mut payload: Payload = Payload::new();

    for (feed_id, data) in resources.iter() {
        payload.values.push(match value.data.get(&data.cmc_id) {
            Some(cmc) => DataFeedResult {
                id: feed_id.clone(),
                value: cmc
                    .quote
                    .get("USD")
                    .unwrap_or(&CmcValue { price: 0.0 })
                    .price,
            },
            None => {
                println!("CMC data feed with id {} is not found", data.cmc_id);
                //TODO: Start reporting error.
                DataFeedResult {
                    id: feed_id.clone(),
                    value: 0.0,
                }
            }
        });
    }

    Ok(payload)
}
