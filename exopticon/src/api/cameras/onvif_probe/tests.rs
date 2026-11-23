use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::post,
};
use oxvif::soap::XmlNode;
use tokio::{net::TcpListener, sync::Notify, task::JoinHandle};
use tower::ServiceExt;

use super::*;

struct CameraFixture {
    base: String,
    port: u16,
    server: JoinHandle<()>,
    keyframe_requested: Arc<Notify>,
}

impl CameraFixture {
    async fn start(media2: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let base = format!("http://{address}");
        let keyframe_requested = Arc::new(Notify::new());
        let app = Router::new()
            .route("/onvif/device_service", post(camera_reply))
            .route("/custom/device", post(camera_reply))
            .route("/onvif/media", post(camera_reply))
            .with_state((
                base.clone(),
                media2,
                Arc::new(AtomicUsize::new(0)),
                Arc::clone(&keyframe_requested),
            ));
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self {
            base,
            port: address.port(),
            server,
            keyframe_requested,
        }
    }
}

impl Drop for CameraFixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn camera_reply(
    State((base, media2, stream_requests, keyframe_requested)): State<(
        String,
        bool,
        Arc<AtomicUsize>,
        Arc<Notify>,
    )>,
    uri: Uri,
    headers: HeaderMap,
    request: String,
) -> Response {
    let soap = XmlNode::parse(&request).unwrap();
    if !media2 {
        if !headers.contains_key(header::AUTHORIZATION) {
            return (
                StatusCode::UNAUTHORIZED,
                [(
                    header::WWW_AUTHENTICATE,
                    r#"Digest realm="test-camera", nonce="fixture", algorithm=MD5, qop="auth""#,
                )],
            )
                .into_response();
        }
        let authorization = headers[header::AUTHORIZATION].to_str().unwrap();
        assert!(authorization.starts_with("Digest "));
        assert!(authorization.contains(r#"username="probe-user""#));
        assert_eq!(
            soap.path(&["Header", "Security", "UsernameToken", "Username"])
                .unwrap()
                .text(),
            "probe-user",
        );
    }
    let action = &soap.child("Body").unwrap().children[0];
    let body = match (uri.path(), action.local_name.as_str()) {
        ("/custom/device", "GetSystemDateAndTime") =>
            "<GetSystemDateAndTimeResponse><SystemDateAndTime/></GetSystemDateAndTimeResponse>".into(),
        ("/onvif/device_service" | "/custom/device", "GetCapabilities") => {
            let media = if media2 {
                String::new()
            } else {
                format!("<Media><XAddr>{base}/onvif/media</XAddr></Media>")
            };
            format!("<GetCapabilitiesResponse><Capabilities>{media}</Capabilities></GetCapabilitiesResponse>")
        }
        ("/onvif/device_service" | "/custom/device", "GetServices") if media2 => format!(
            "<GetServicesResponse><Service><Namespace>http://www.onvif.org/ver20/media/wsdl</Namespace><XAddr>{base}/onvif/media</XAddr></Service></GetServicesResponse>"
        ),
        ("/onvif/device_service", "GetDeviceInformation") => String::from(
            "<GetDeviceInformationResponse><Manufacturer>Test Camera Co</Manufacturer><Model>Test Model</Model><FirmwareVersion>1.2</FirmwareVersion><SerialNumber>12345</SerialNumber></GetDeviceInformationResponse>",
        ),
        ("/onvif/media", "GetProfiles") => {
            let config = if media2 {
                assert_eq!(action.child("Type").unwrap().text(), "All");
                r#"<Configurations><VideoEncoder token="video"/><PTZ token="ptz"/></Configurations>"#
            } else {
                r#"<VideoEncoderConfiguration token="video"/><PTZConfiguration token="ptz"/>"#
            };
            format!(
                r#"<GetProfilesResponse>
                      <Profiles token="main"><Name>Main stream</Name>{config}</Profiles>
                      <Profiles token="sub"><Name>Unavailable stream</Name></Profiles>
                   </GetProfilesResponse>"#
            )
        }
        ("/onvif/media", "GetStreamUri") if action.child("ProfileToken").unwrap().text() == "main" => {
            let uri = match stream_requests.fetch_add(1, Ordering::Relaxed) {
                0 => "<Uri>rtsp://127.0.0.1:554/main</Uri>",
                1 => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
                _ => "<Uri>rtsp://stream.example:8554/fresh</Uri>",
            };
            let media_uri = if media2 { uri.to_string() } else { format!("<MediaUri>{uri}</MediaUri>") };
            format!("<GetStreamUriResponse>{media_uri}</GetStreamUriResponse>")
        }
        ("/onvif/media", "GetSnapshotUri") if !media2 && action.child("ProfileToken").unwrap().text() == "main" => {
            "<GetSnapshotUriResponse><MediaUri><Uri>http://camera-user:camera-password@127.0.0.1/snapshot</Uri></MediaUri></GetSnapshotUriResponse>".into()
        }
        ("/onvif/media", "SetSynchronizationPoint") => {
            assert_eq!(action.child("ProfileToken").unwrap().text(), "main");
            assert!(request.contains(if media2 { "/ver20/media/wsdl" } else { "/ver10/media/wsdl" }));
            keyframe_requested.notify_one();
            "<SetSynchronizationPointResponse/>".into()
        }
        ("/onvif/media", "GetVideoEncoderConfiguration") => {
            assert_eq!(action.child("ConfigurationToken").unwrap().text(), "video");
            format!("<GetVideoEncoderConfigurationResponse>{ENCODER}</GetVideoEncoderConfigurationResponse>")
        }
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    (
        StatusCode::OK,
        format!(
            r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body>{body}</s:Body></s:Envelope>"#
        ),
    ).into_response()
}

const ENCODER: &str = r#"<Configuration token="video">
    <Name>Main encoder</Name><UseCount>1</UseCount><Encoding>H264</Encoding>
    <Resolution><Width>1920</Width><Height>1080</Height></Resolution>
    <Quality>5</Quality>
    <RateControl><FrameRateLimit>25</FrameRateLimit><EncodingInterval>1</EncodingInterval><BitrateLimit>4096</BitrateLimit></RateControl>
</Configuration>"#;

#[tokio::test]
async fn probe_prefers_the_advertised_endpoint_and_keeps_partial_profile_results() {
    let camera = CameraFixture::start(false).await;
    let request = ProbeRequest {
        host: "localhost".into(),
        port: Some(1),
        username: "probe-user".into(),
        password: "probe-password".into(),
    };
    let target = resolve_target(&request.host).await.unwrap();
    let advertised = format!("{}/onvif/device_service", camera.base);
    let result = probe_endpoints(
        &request,
        &target,
        1,
        [
            "http://192.0.2.1/onvif/device_service".into(),
            advertised.clone(),
        ]
        .into_iter(),
    )
    .await
    .unwrap();

    assert_eq!(result.host, "localhost");
    assert_eq!(result.port, camera.port);
    assert_eq!(result.device_service_url, advertised);
    assert_eq!(result.device.unwrap().model, "Test Model");
    assert_eq!(result.profiles.len(), 2);
    let main = &result.profiles[0];
    assert_eq!(main.token, "main");
    assert!(main.ptz_supported);
    assert!(main.reuse_credentials);
    assert_eq!(
        main.snapshot_url.as_deref(),
        Some("http://127.0.0.1/snapshot")
    );
    let video = main.video.as_ref().unwrap();
    assert_eq!(
        (
            video.codec.as_str(),
            video.width,
            video.height,
            video.frame_rate
        ),
        ("H264", 1920, 1080, Some(25.0))
    );
    assert_eq!(result.profiles[1].token, "sub");
    assert!(result.profiles[1].rtsp_url.is_none());
}

#[tokio::test]
async fn probe_api_uses_fallback_port_and_reads_media2_profiles() {
    let camera = CameraFixture::start(true).await;
    let app = Router::new().route("/onvif/probe", post(probe));
    let body = serde_json::json!({ "host": "127.0.0.1", "port": camera.port });
    let response = app
        .oneshot(
            Request::post("/onvif/probe")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body["deviceServiceUrl"],
        format!("{}/onvif/device_service", camera.base)
    );
    assert_eq!(body["profiles"][0]["rtspUrl"], "rtsp://127.0.0.1:554/main");
    assert!(body["profiles"][0]["snapshotUrl"].is_null());
    assert!(body["profiles"][1]["rtspUrl"].is_null());
}

#[tokio::test]
async fn capture_actor_resolves_a_fresh_uri_and_requests_keyframes_for_the_same_profile() {
    for media2 in [false, true] {
        let fixture = CameraFixture::start(media2).await;
        let (_dir, db) = crate::db::tests::configured_service();
        let mut camera = db.fetch_camera_row("front").unwrap();
        camera.username = "probe-user".into();
        camera.password = "pass%:/word".into();
        let (handle, actor) = crate::onvif_actor::actor(
            camera,
            format!("{}/custom/device", fixture.base),
            "main".into(),
        );
        let task = tokio::spawn(actor);

        let first = handle.stream_uri().await.unwrap();
        assert_eq!(first.host_str(), Some("127.0.0.1"));
        assert!(handle.stream_uri().await.is_err());
        let next = handle.stream_uri().await.unwrap();
        assert_eq!(
            next.as_str(),
            "rtsp://probe-user:pass%25%3A%2Fword@stream.example:8554/fresh"
        );

        handle.keyframe_requests.request();
        timeout(
            Duration::from_secs(1),
            fixture.keyframe_requested.notified(),
        )
        .await
        .unwrap();
        drop(handle);
        task.await.unwrap();
    }
}
