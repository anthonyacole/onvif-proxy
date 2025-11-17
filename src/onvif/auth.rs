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
            r#"<wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd" xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
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

    pub fn add_to_soap_header(&self, soap_xml: &str) -> String {
        let security_header = self.generate_header();

        // If SOAP envelope already has a header, insert Security into it
        if soap_xml.contains("<SOAP-ENV:Header>") {
            soap_xml.replace(
                "<SOAP-ENV:Header>",
                &format!("<SOAP-ENV:Header>{}", security_header),
            )
        } else {
            // Insert new header after Envelope opening tag
            soap_xml.replace(
                "<SOAP-ENV:Envelope",
                &format!(
                    "<SOAP-ENV:Envelope xmlns:SOAP-ENV=\"http://www.w3.org/2003/05/soap-envelope\">\n<SOAP-ENV:Header>{}</SOAP-ENV:Header>",
                    security_header
                ).as_str(),
            ).replace(
                ">\n<SOAP-ENV:Header>",
                ">"
            )
        }
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
