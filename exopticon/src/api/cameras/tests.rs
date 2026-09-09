use std::{collections::VecDeque, sync::Arc};

use axum::{
    Router,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use oxvif::soap::{XmlNode, security::compute_digest};
use tokio::{
    net::TcpListener,
    sync::{Mutex, mpsc},
    task::JoinHandle,
};

use super::{UserError, move_camera, onvif_device_url};

struct RecordedRequest {
    headers: HeaderMap,
    body: XmlNode,
}

struct TestCamera {
    camera: crate::db::cameras::Camera,
    requests: mpsc::UnboundedReceiver<RecordedRequest>,
    server: JoinHandle<()>,
}

impl Drop for TestCamera {
    fn drop(&mut self) {
        self.server.abort();
    }
}

impl TestCamera {
    async fn start(responses: Vec<(StatusCode, &'static str)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
        let (sender, requests) = mpsc::unbounded_channel();
        let app = Router::new().route(
            "/onvif/device_service",
            post(move |headers: HeaderMap, body: String| {
                let responses = Arc::clone(&responses);
                let sender = sender.clone();
                async move {
                    sender
                        .send(RecordedRequest {
                            headers,
                            body: XmlNode::parse(&body).unwrap(),
                        })
                        .unwrap();
                    let (status, body) = responses.lock().await.pop_front().unwrap();
                    (
                        status,
                        [("content-type", "application/soap+xml")],
                        format!(
                            r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"><s:Body>{body}</s:Body></s:Envelope>"#
                        ),
                    )
                }
            }),
        );
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            camera: crate::db::cameras::Camera {
                name: "test-camera".to_string(),
                display_name: "Test Camera".to_string(),
                storage_group_name: "default".to_string(),
                ip: "127.0.0.1".to_string(),
                onvif_port: i32::from(port),
                mac: String::new(),
                username: "camera<&user".to_string(),
                password: "camera-password".to_string(),
                rtsp_url: "rtsp://capture.example/live".to_string(),
                ptz_type: "onvif_relative".to_string(),
                ptz_profile_token: "profile<&token".to_string(),
                enabled: true,
                ptz_x_step_size: 25,
                ptz_y_step_size: 40,
            },
            requests,
            server,
        }
    }
}

#[test]
fn device_url_preserves_host_port_and_ipv6_support() {
    for (host, expected_host) in [
        ("192.0.2.10", "192.0.2.10"),
        ("camera.example.com", "camera.example.com"),
        ("2001:db8::1", "[2001:db8::1]"),
        ("[2001:db8::1]", "[2001:db8::1]"),
    ] {
        assert_eq!(
            onvif_device_url(host, 8899),
            format!("http://{expected_host}:8899/onvif/device_service")
        );
    }
}

#[tokio::test]
async fn relative_moves_preserve_directions_profile_and_credentials() {
    let mut fixture =
        TestCamera::start(vec![(StatusCode::OK, "<tptz:RelativeMoveResponse/>"); 4]).await;

    for (direction, x, y) in [
        ("left", "-0.25", "0"),
        ("right", "0.25", "0"),
        ("up", "0", "0.4"),
        ("down", "0", "-0.4"),
    ] {
        move_camera(&fixture.camera, direction).await.unwrap();
        let request = fixture.requests.try_recv().unwrap();
        assert!(
            request.headers["content-type"]
                .to_str()
                .unwrap()
                .contains("http://www.onvif.org/ver20/ptz/wsdl/RelativeMove")
        );
        let movement = request.body.path(&["Body", "RelativeMove"]).unwrap();
        assert_eq!(
            movement.child("ProfileToken").unwrap().text(),
            fixture.camera.ptz_profile_token
        );
        let pan_tilt = movement.path(&["Translation", "PanTilt"]).unwrap();
        assert_eq!(pan_tilt.attr("x"), Some(x));
        assert_eq!(pan_tilt.attr("y"), Some(y));

        let token = request
            .body
            .path(&["Header", "Security", "UsernameToken"])
            .unwrap();
        assert_eq!(
            token.child("Username").unwrap().text(),
            fixture.camera.username
        );
        let nonce = STANDARD
            .decode(token.child("Nonce").unwrap().text())
            .unwrap();
        let digest = compute_digest(
            &nonce,
            token.child("Created").unwrap().text(),
            &fixture.camera.password,
        );
        assert_eq!(
            token.child("Password").unwrap().text(),
            STANDARD.encode(digest)
        );
    }
    assert!(fixture.requests.try_recv().is_err());
}

#[tokio::test]
async fn invalid_direction_does_not_contact_camera() {
    let mut fixture = TestCamera::start(vec![]).await;
    assert!(matches!(
        move_camera(&fixture.camera, "diagonal").await,
        Err(UserError::Validation(_))
    ));
    assert!(fixture.requests.try_recv().is_err());
}

#[tokio::test]
async fn relative_move_failures_remain_errors() {
    for response in [
        (StatusCode::OK, "<tptz:GetStatusResponse/>"),
        (StatusCode::UNAUTHORIZED, ""),
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "<s:Fault><s:Code><s:Value>s:Receiver</s:Value></s:Code><s:Reason><s:Text>Move failed</s:Text></s:Reason></s:Fault>",
        ),
    ] {
        let fixture = TestCamera::start(vec![response]).await;
        assert!(matches!(
            move_camera(&fixture.camera, "up").await,
            Err(UserError::InternalError(message)) if message == "relative move failed"
        ));
    }
}
