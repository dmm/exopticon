/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2024-2026 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */
use std::env;

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Basic},
    typed_header::TypedHeaderRejection,
};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

const BASIC_AUTH_CHALLENGE: &str = "Basic realm=\"metrics\"";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricsCredentials {
    username_hash: [u8; 32],
    password_hash: [u8; 32],
}

impl MetricsCredentials {
    #[must_use]
    pub fn new(username: &str, password: &str) -> Self {
        Self {
            username_hash: Sha256::digest(username.as_bytes()).into(),
            password_hash: Sha256::digest(password.as_bytes()).into(),
        }
    }

    #[must_use]
    pub fn from_env() -> Option<Self> {
        Self::from_values(
            env::var("EXOPTICON_METRICS_ENABLED").ok().as_deref(),
            env::var("EXOPTICON_METRICS_USERNAME").ok().as_deref(),
            env::var("EXOPTICON_METRICS_PASSWORD").ok().as_deref(),
        )
    }

    fn from_values(
        enabled: Option<&str>,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Option<Self> {
        if enabled != Some("true") {
            return None;
        }

        let Some(username) = username.filter(|value| !value.is_empty()) else {
            error!("Metrics enabled but EXOPTICON_METRICS_USERNAME is not set; disabling /metrics");
            return None;
        };

        let Some(password) = password.filter(|value| !value.is_empty()) else {
            error!("Metrics enabled but EXOPTICON_METRICS_PASSWORD is not set; disabling /metrics");
            return None;
        };

        Some(Self::new(username, password))
    }

    fn matches(&self, username: &str, password: &str) -> bool {
        let user_ok = Sha256::digest(username.as_bytes())
            .as_slice()
            .ct_eq(&self.username_hash);
        let pass_ok = Sha256::digest(password.as_bytes())
            .as_slice()
            .ct_eq(&self.password_hash);
        bool::from(user_ok & pass_ok)
    }
}

fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, BASIC_AUTH_CHALLENGE)],
    )
        .into_response()
}

pub async fn metrics_auth_middleware(
    State(credentials): State<MetricsCredentials>,
    auth_header: Result<TypedHeader<Authorization<Basic>>, TypedHeaderRejection>,
    req: Request,
    next: Next,
) -> Response {
    let Ok(TypedHeader(auth_header)) = auth_header else {
        return unauthorized_response();
    };

    if credentials.matches(auth_header.username(), auth_header.password()) {
        next.run(req).await
    } else {
        unauthorized_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode, header},
        middleware,
        routing::get,
    };
    use base64::prelude::{BASE64_STANDARD, Engine as _};
    use tower::ServiceExt;

    use super::{BASIC_AUTH_CHALLENGE, MetricsCredentials, metrics_auth_middleware};

    fn metrics_router(credentials: MetricsCredentials) -> Router {
        Router::new().route(
            "/metrics",
            get(|| async { "ok" }).layer(middleware::from_fn_with_state(
                credentials,
                metrics_auth_middleware,
            )),
        )
    }

    fn basic_header(username: &str, password: &str) -> String {
        format!(
            "Basic {}",
            BASE64_STANDARD.encode(format!("{username}:{password}"))
        )
    }

    #[test]
    fn metrics_credentials_require_enabled_and_nonempty_values() {
        assert_eq!(
            MetricsCredentials::from_values(None, Some("prom"), Some("secret")),
            None
        );
        assert_eq!(
            MetricsCredentials::from_values(Some("false"), Some("prom"), Some("secret")),
            None
        );
        assert_eq!(
            MetricsCredentials::from_values(Some("true"), Some(""), Some("secret")),
            None
        );
        assert_eq!(
            MetricsCredentials::from_values(Some("true"), Some("prom"), None),
            None
        );
        assert_eq!(
            MetricsCredentials::from_values(Some("true"), Some("prom"), Some("secret")),
            Some(MetricsCredentials::new("prom", "secret"))
        );
    }

    #[tokio::test]
    async fn missing_or_malformed_auth_header_returns_unauthorized_with_basic_challenge() {
        let app = metrics_router(MetricsCredentials::new("prom", "secret"));

        for header_value in [None, Some("Bearer token"), Some("Basic !!!not-base64!!!")] {
            let mut builder = Request::builder().uri("/metrics");
            if let Some(value) = header_value {
                builder = builder.header(header::AUTHORIZATION, value);
            }
            let response = app
                .clone()
                .oneshot(builder.body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(
                response.headers().get(header::WWW_AUTHENTICATE).unwrap(),
                BASIC_AUTH_CHALLENGE
            );
        }
    }

    #[tokio::test]
    async fn valid_basic_credentials_allow_access_and_wrong_credentials_reject() {
        let app = metrics_router(MetricsCredentials::new("prom", "secret"));

        let ok = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .header(header::AUTHORIZATION, basic_header("prom", "secret"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK);

        for (user, pass) in [
            ("prom", "wrong"),
            ("wrong", "secret"),
            ("prom", "secre"),
            ("prom", "secret2"),
        ] {
            let rejected = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/metrics")
                        .header(header::AUTHORIZATION, basic_header(user, pass))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(
                rejected.headers().get(header::WWW_AUTHENTICATE).unwrap(),
                BASIC_AUTH_CHALLENGE
            );
        }
    }
}
