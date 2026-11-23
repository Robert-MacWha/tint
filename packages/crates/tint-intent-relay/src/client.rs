use alloy_primitives::B256;
use reqwest::{Client, Url};
use tint_intent_policy::intent::{CancelKey, Intent};

use crate::server::{CancelRequest, IntentFilter};

/// A client for the [relay](crate::server).
#[derive(Clone, Debug)]
pub struct RelayClient {
    url: Url,
    http: Client,
}

impl RelayClient {
    #[must_use]
    pub fn new(url: Url) -> Self {
        Self {
            url,
            http: Client::new(),
        }
    }

    /// Posts `intent`, returning its id.
    ///
    /// # Errors
    ///
    /// Errors if the request fails or the relay rejects the intent.
    pub async fn post(&self, intent: &Intent) -> Result<B256, reqwest::Error> {
        self.http
            .post(self.intents_url())
            .json(intent)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    /// Lists the open intents matching `filter`.
    ///
    /// # Errors
    ///
    /// Errors if the request fails.
    pub async fn list(&self, filter: &IntentFilter) -> Result<Vec<Intent>, reqwest::Error> {
        self.http
            .get(self.intents_url())
            .query(filter)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    /// Removes intent `id` from the relay. This doesn't cancel it on-chain.
    ///
    /// # Errors
    ///
    /// Errors if the request fails, or `key` doesn't cancel intent `id`.
    pub async fn cancel(&self, id: B256, key: &CancelKey) -> Result<(), reqwest::Error> {
        let request = CancelRequest {
            cancel_token: key.token(),
        };
        self.http
            .delete(format!("{}/{id}", self.intents_url()))
            .json(&request)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    fn intents_url(&self) -> String {
        format!("{}/intents", self.url.as_str().trim_end_matches('/'))
    }
}
