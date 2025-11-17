use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use sha1::{Sha1, Digest};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct WsSecurityAuth {
    pub username: String,
    pub password: String,
}

impl WsSecurityAuth {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    pub fn generate_header(&self) -> String {
        let nonce_bytes = Uuid::new_v4().as_bytes().to_vec();
        let nonce_base64 = BASE64.encode(&nonce_bytes);

        let created = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        // Calculate password digest: Base64(SHA1(nonce + created + password))
        let mut hasher = Sha1::new();
        hasher.update(&nonce_bytes);
        hasher.update(created.as_bytes());
        hasher.update(self.password.as_bytes());
        let digest = hasher.finalize();
        let password_digest = BASE64.encode(digest);

        format!(
            r#"<wsse:Security>
  <wsse:UsernameToken>
    <wsse:Username>{}</wsse:Username>
    <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">{}</wsse:Password>
    <wsse:Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">{}</wsse:Nonce>
    <wsu:Created>{}</wsu:Created>
  </wsse:UsernameToken>
</wsse:Security>"#,
            self.username, password_digest, nonce_base64, created
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_header() {
        let auth = WsSecurityAuth::new("admin".to_string(), "password".to_string());
        let header = auth.generate_header();

        assert!(header.contains("<wsse:Username>admin</wsse:Username>"));
        assert!(header.contains("<wsse:Password"));
        assert!(header.contains("<wsse:Nonce"));
        assert!(header.contains("<wsu:Created"));
    }
}
