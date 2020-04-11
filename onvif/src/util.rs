//! Onvif api utilities
use chrono::{Duration, SecondsFormat, Utc};
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use hyper::{Body, Client, Request};
use rand::Rng;
use tokio::stream::StreamExt;

use crate::error::Error;

/// A Struct representing an onvif password digest
struct PasswordDigest {
    /// base64 encoded digest
    pub digest: String,
    /// base64 encoded nonce
    pub nonce: String,
    /// base64 encoded timestamp
    pub timestamp: String,
}

/// Returns `PasswordDigest` generated from given password and
/// offset. The current system time is used for the timestamp.
fn generate_password_digest(password: &str, offset: Duration) -> Result<PasswordDigest, Error> {
    let timestamp = match Utc::now().checked_add_signed(offset) {
        Some(datetime) => datetime.to_rfc3339_opts(SecondsFormat::Millis, true),
        None => return Err(Error::InvalidArgument),
    };

    let mut rng = rand::thread_rng();
    let nonce: [u8; 16] = rng.gen();

    let mut hasher = Sha1::new();
    hasher.input(&nonce);
    hasher.input(timestamp.as_bytes());
    hasher.input(password.as_bytes());
    let mut hash_bytes = vec![0; hasher.output_bytes()];
    hasher.result(&mut hash_bytes);

    Ok(PasswordDigest {
        digest: base64::encode(hash_bytes.as_slice()),
        nonce: base64::encode(&nonce),
        timestamp,
    })
}

/// Returns security block as a String. If username or password are
/// blank the String returned is also blank.
fn generate_security_block(username: &str, password: &str) -> Result<String, Error> {
    if username == "" && password == "" {
        return Ok(String::new());
    }

    let digest = generate_password_digest(password, Duration::zero())?;
    Ok(format!(
        r#"
     <Security s:mustUnderstand="1"
               xmlns="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd">
       <UsernameToken>
         <Username>{}</Username>
         <Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">{}</Password>
         <Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">{}</Nonce>
         <Created xmlns="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">{}</Created>
       </UsernameToken>
     </Security>"#,
        username, digest.digest, digest.nonce, digest.timestamp
    ))
}

/// Returns soap request header as a String
///
/// # Arguments
///
/// * `username` - username for authentication
/// * `password` - password for authentication
///
pub fn envelope_header(username: &str, password: &str) -> Result<String, Error> {
    let security_block = generate_security_block(username, password)?;
    Ok(format!(
        r#"
     <s:Envelope
      xmlns:s="http://www.w3.org/2003/05/soap-envelope"
      xmlns:a="http://www.w3.org/2005/08/addressing"
     >

  <s:Header>{}</s:Header>
  <s:Body xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">"#,
        security_block
    ))
}

/// Returns String for closing tags of soap request
pub fn envelope_footer() -> String {
    String::from("</s:Body></s:Envelope>")
}

/// Returns Future resolving to a response Result
///
/// # Arguments
///
/// * `url` - a url to submit to request to
/// * `body` - request body
///
pub async fn soap_request(url: &str, body: String) -> Result<Vec<u8>, Error> {
    let client = Client::new();

    let url: hyper::Uri = match url.parse() {
        Ok(u) => u,
        Err(_) => return Err(Error::InvalidArgument),
    };

    let req = match Request::builder()
        .method("POST")
        .uri(url)
        .header("Content-Type", "application/soap+xml")
        .body(Body::from(body))
    {
        Ok(req) => req,
        Err(_) => return Err(Error::InvalidArgument),
    };

    let mut res = client.request(req).await?;
    let body = res.body_mut();
    let mut output = Vec::new();

    while let Some(chunk) = body.next().await {
        let bytes = chunk?;
        output.extend(&bytes[..]);
    }

    Ok(output)
}
