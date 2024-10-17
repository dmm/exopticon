/// size of webrtc udp send/recv buffers if not set with env variable
const fn default_buffer_size() -> usize {
    2_097_152
}
#[derive(Deserialize, Debug)]
pub struct Config {
    pub database_url: String,
    pub domain: String,
    pub exopticon_name: String,
    pub exopticon_short_name: String,
    pub metrics_enabled: bool,
    pub metrics_username: Option<String>,
    pub metrics_password: Option<String>,
    pub webrtc_ips: Vec<String>,
    #[serde(default = "default_buffer_size")]
    pub webrtc_buffer_size: usize,
}
