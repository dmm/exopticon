/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2023 David Matthew Mattli <dmm@mattli.us>
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

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::{extract::State, Json, Router};
use serde::{Deserialize, Serialize};
use str0m::{change::SdpOffer, Candidate, Rtc};

use crate::AppState;

use super::UserError;

#[derive(Debug, Serialize, Deserialize)]
pub struct RtcSessionDescriptionInit {
    #[serde(rename(serialize = "type", deserialize = "type"))]
    kind: String,
    sdp: String,
}

pub async fn connect(
    State(state): State<AppState>,
    Json(offer): Json<RtcSessionDescriptionInit>,
) -> Result<Json<RtcSessionDescriptionInit>, UserError> {
    let sdp_offer = SdpOffer::from_sdp_string(&offer.sdp).or_else(|_err| {
        // Parsing the sdp failed
        Err(UserError::Validation(
            "Invalid Offer, parsing sdp failed".to_string(),
        ))
    })?;

    let mut rtc = Rtc::builder()
        // Uncomment this to see statistics
        // .set_stats_interval(Some(Duration::from_secs(1)))
        // .set_ice_lite(true)
        .build();

    // Add the shared UDP socket as a host candidate
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 5, 187)), 4000);
    let candidate = Candidate::host(addr).expect("a host candidate");
    rtc.add_local_candidate(candidate);

    // Create an SDP Answer.
    let answer = rtc.sdp_api().accept_offer(sdp_offer).or_else(|_| {
        Err(UserError::Validation(
            "Unable to generate webrtc answer".to_string(),
        ))
    })?;

    // The Rtc instance is shipped off to the main run loop.
    state.rtc_sender.send(rtc).await.or_else(|err| {
        error!("Unable to pass rtc instance to main loop: {}", err);
        Err(UserError::InternalError)
    })?;

    Ok(Json(RtcSessionDescriptionInit {
        kind: "answer".to_string(),
        sdp: answer.to_sdp_string(),
    }))
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route("/connect", axum::routing::post(connect))
}
