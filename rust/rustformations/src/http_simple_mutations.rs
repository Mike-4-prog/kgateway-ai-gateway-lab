use anyhow::{Context, Result};
use envoy_proxy_dynamic_modules_rust_sdk::{
    EnvoyBuffer, EnvoyHttpFilter, HttpFilter, HttpFilterConfig,
};
use minijinja::Environment;
use once_cell::sync::Lazy;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use transformations::{
    LocalTransform, LocalTransformationConfig, TransformationError, TransformationOps,
};

#[cfg(test)]
use mockall::*;

static EMPTY_MAP: Lazy<HashMap<String, String>> = Lazy::new(HashMap::new);

#[derive(Clone)]
pub struct FilterConfig {
    transformations: LocalTransformationConfig,
    env: Environment<'static>,
}

/* =========================
   TRANSFORMATION OPS
========================= */

struct EnvoyTransformationOps<'a> {
    envoy_filter: &'a mut dyn EnvoyHttpFilter,
    used_request_from_received: Option<bool>,
    used_response_from_received: Option<bool>,
}

impl<'a> EnvoyTransformationOps<'a> {
    fn new(envoy_filter: &'a mut dyn EnvoyHttpFilter) -> Self {
        Self {
            envoy_filter,
            used_request_from_received: None,
            used_response_from_received: None,
        }
    }
}

impl TransformationOps for EnvoyTransformationOps<'_> {
    fn add_request_header(&mut self, key: &str, value: &[u8]) -> bool {
        self.envoy_filter.set_request_header(key, value)
    }

    fn set_request_header(&mut self, key: &str, value: &[u8]) -> bool {
        self.envoy_filter.set_request_header(key, value)
    }

    fn remove_request_header(&mut self, key: &str) -> bool {
        self.envoy_filter.remove_request_header(key)
    }

    fn parse_request_json_body(&mut self) -> Result<JsonValue> {
        let body = self.get_request_body();
        if body.is_empty() {
            return Ok(JsonValue::Null);
        }
        serde_json::from_slice(&body).context("failed to parse request body")
    }

    fn get_request_body(&mut self) -> Vec<u8> {
        let mut buffers = self.envoy_filter.get_buffered_request_body();

        if buffers.is_none() {
            buffers = self.envoy_filter.get_received_request_body();
            if buffers.is_some() {
                self.used_request_from_received = Some(true);
            }
        }

        match buffers {
            None => vec![],
            Some(bufs) => bufs.iter().map(|b| b.as_slice()).collect::<Vec<_>>().concat(),
        }
    }

    fn drain_request_body(&mut self, size: usize) -> bool {
        let use_received = self.used_request_from_received.unwrap_or(false);

        if use_received {
            self.envoy_filter.drain_received_request_body(size)
        } else {
            self.envoy_filter.drain_buffered_request_body(size)
        }
    }

    fn append_request_body(&mut self, data: &[u8]) -> bool {
        let use_received = self.used_request_from_received.unwrap_or(false);

        if use_received {
            self.envoy_filter.append_received_request_body(data)
        } else {
            self.envoy_filter.append_buffered_request_body(data)
        }
    }

    fn add_response_header(&mut self, key: &str, value: &[u8]) -> bool {
        self.envoy_filter.set_response_header(key, value)
    }

    fn set_response_header(&mut self, key: &str, value: &[u8]) -> bool {
        self.envoy_filter.set_response_header(key, value)
    }

    fn remove_response_header(&mut self, key: &str) -> bool {
        self.envoy_filter.remove_response_header(key)
    }

    fn parse_response_json_body(&mut self) -> Result<JsonValue> {
        let body = self.get_response_body();
        if body.is_empty() {
            return Ok(JsonValue::Null);
        }
        serde_json::from_slice(&body).context("failed to parse response body")
    }

    fn get_response_body(&mut self) -> Vec<u8> {
        let mut buffers = self.envoy_filter.get_buffered_response_body();

        if buffers.is_none() {
            buffers = self.envoy_filter.get_received_response_body();
            if buffers.is_some() {
                self.used_response_from_received = Some(true);
            }
        }

        match buffers {
            None => vec![],
            Some(bufs) => bufs.iter().map(|b| b.as_slice()).collect::<Vec<_>>().concat(),
        }
    }

    fn drain_response_body(&mut self, size: usize) -> bool {
        let use_received = self.used_response_from_received.unwrap_or(false);

        if use_received {
            self.envoy_filter.drain_received_response_body(size)
        } else {
            self.envoy_filter.drain_buffered_response_body(size)
        }
    }

    fn append_response_body(&mut self, data: &[u8]) -> bool {
        let use_received = self.used_response_from_received.unwrap_or(false);

        if use_received {
            self.envoy_filter.append_received_response_body(data)
        } else {
            self.envoy_filter.append_buffered_response_body(data)
        }
    }
}

/* =========================
   CONFIG
========================= */

impl FilterConfig {
    pub fn new(filter_config: &str) -> Option<Self> {
        let config: LocalTransformationConfig = serde_json::from_str(filter_config).ok()?;

        let env = transformations::jinja::create_env_with_templates(&config).ok()?;

        Some(Self {
            transformations: config,
            env,
        })
    }
}

pub type PerRouteConfig = FilterConfig;

/* =========================
   FILTER FACTORY
========================= */

impl<EHF: EnvoyHttpFilter> HttpFilterConfig<EHF> for FilterConfig {
    fn new_http_filter(&mut self, _envoy: &mut EHF) -> Box<dyn HttpFilter<EHF>> {
        Box::new(Filter {
            filter_config: self.clone(),
            per_route_config: None,
            request_headers_map: None,
        })
    }
}

/* =========================
   FILTER STATE
========================= */

pub struct Filter {
    filter_config: FilterConfig,
    per_route_config: Option<Box<PerRouteConfig>>,
    request_headers_map: Option<HashMap<String, String>>,
}

/* =========================
   FILTER HELPERS
========================= */

impl Filter {
    fn get_env(&self) -> &Environment<'static> {
        self.get_per_route_config()
            .map(|c| &c.env)
            .unwrap_or(&self.filter_config.env)
    }

    fn set_per_route_config<EHF: EnvoyHttpFilter>(&mut self, envoy: &mut EHF) {
        if self.per_route_config.is_some() {
            return;
        }

        if let Some(cfg) = envoy.get_most_specific_route_config() {
            if let Some(cfg) = cfg.downcast_ref::<PerRouteConfig>() {
                self.per_route_config = Some(Box::new(cfg.clone()));
            }
        }
    }

    fn get_per_route_config(&self) -> Option<&PerRouteConfig> {
        self.per_route_config.as_deref()
    }

    fn create_headers_map(
        &self,
        headers: Vec<(EnvoyBuffer, EnvoyBuffer)>,
    ) -> HashMap<String, String> {
        headers
            .into_iter()
            .filter_map(|(k, v)| {
                Some((
                    std::str::from_utf8(k.as_slice()).ok()?.to_string(),
                    std::str::from_utf8(v.as_slice()).ok()?.to_string(),
                ))
            })
            .collect()
    }

    fn populate_request_headers_map(&mut self, headers: Vec<(EnvoyBuffer, EnvoyBuffer)>) {
        if self.request_headers_map.is_none() {
            self.request_headers_map = Some(self.create_headers_map(headers));
        }
    }

    fn get_request_headers_map(&self) -> &HashMap<String, String> {
        self.request_headers_map.as_ref().unwrap_or(&EMPTY_MAP)
    }

    fn get_request_transform(&self) -> &Option<LocalTransform> {
        self.get_per_route_config()
            .map(|c| &c.transformations.request)
            .unwrap_or(&self.filter_config.transformations.request)
    }

    fn get_response_transform(&self) -> &Option<LocalTransform> {
        self.get_per_route_config()
            .map(|c| &c.transformations.response)
            .unwrap_or(&self.filter_config.transformations.response)
    }

    fn has_request_transform(&self) -> bool {
        self.get_request_transform()
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false)
    }

    fn has_response_transform(&self) -> bool {
        self.get_response_transform()
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false)
    }
}