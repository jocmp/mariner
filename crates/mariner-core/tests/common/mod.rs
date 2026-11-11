//! A stubbed network for account tests.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mariner_core::{Account, Database, Kind};
use mariner_feeds::fetch::{Fetch, FetchError, Redirect, Request, Response};

/// A local account in a temporary directory, fetching from `network`.
pub async fn local_account(network: &Network) -> (tempfile::TempDir, Account<Network>) {
    let dir = tempfile::tempdir().unwrap();
    let database = Database::open(dir.path().join("local.db"), Kind::Account)
        .await
        .unwrap();
    (dir, Account::local(database, network.clone()))
}

/// Serves stubbed responses (404 for anything unknown), follows stubbed
/// redirects, and records every request. Clones share state, so a test can
/// change responses between calls.
#[derive(Clone, Default)]
pub struct Network {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    routes: HashMap<String, Route>,
    requests: Vec<Request>,
}

#[derive(Clone)]
enum Route {
    Response {
        status: u16,
        headers: Vec<(String, String)>,
        body: String,
    },
    Redirect {
        status: u16,
        to: String,
    },
    Unreachable,
}

impl Network {
    /// Serves `body` at `url` with status 200.
    pub fn with(self, url: &str, body: &str) -> Self {
        self.respond(url, 200, &[], body);
        self
    }

    pub fn respond(&self, url: &str, status: u16, headers: &[(&str, &str)], body: &str) {
        let headers = headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        self.route(
            url,
            Route::Response {
                status,
                headers,
                body: body.to_string(),
            },
        );
    }

    pub fn redirect(&self, from: &str, status: u16, to: &str) {
        self.route(
            from,
            Route::Redirect {
                status,
                to: to.to_string(),
            },
        );
    }

    /// Fails to connect to `url`.
    pub fn unreachable(&self, url: &str) {
        self.route(url, Route::Unreachable);
    }

    pub fn requests(&self) -> Vec<Request> {
        self.state.lock().unwrap().requests.clone()
    }

    pub fn requested_urls(&self) -> Vec<String> {
        self.requests()
            .into_iter()
            .map(|request| request.url)
            .collect()
    }

    pub fn clear_requests(&self) {
        self.state.lock().unwrap().requests.clear();
    }

    fn route(&self, url: &str, route: Route) {
        self.state
            .lock()
            .unwrap()
            .routes
            .insert(url.to_string(), route);
    }
}

impl Fetch for Network {
    async fn fetch(&self, request: &Request) -> Result<Response, FetchError> {
        let mut state = self.state.lock().unwrap();
        state.requests.push(request.clone());
        let mut url = request.url.clone();
        let mut redirects = Vec::new();
        loop {
            match state.routes.get(&url).cloned() {
                Some(Route::Redirect { status, to }) => {
                    redirects.push(Redirect {
                        status,
                        from: url.clone(),
                        to: to.clone(),
                    });
                    url = to;
                }
                Some(Route::Response {
                    status,
                    headers,
                    body,
                }) => {
                    return Ok(Response {
                        url,
                        status,
                        headers,
                        body: body.into_bytes(),
                        redirects,
                    });
                }
                Some(Route::Unreachable) => return Err(FetchError::Connect(url)),
                None => {
                    return Ok(Response {
                        url,
                        status: 404,
                        redirects,
                        ..Response::default()
                    });
                }
            }
        }
    }
}
