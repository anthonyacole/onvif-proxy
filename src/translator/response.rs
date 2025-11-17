use anyhow::Result;
use crate::translator::reolink::ReolinkEventTranslator;

pub struct ResponseTranslator;

impl ResponseTranslator {
    pub fn translate(xml: &str, camera_model: &str, quirks: &[String]) -> Result<String> {
        match camera_model {
            "reolink" => ReolinkEventTranslator::translate_response(xml, quirks),
            _ => {
                tracing::warn!("Unknown camera model: {}, no translation applied", camera_model);
                Ok(xml.to_string())
            }
        }
    }

    pub fn ensure_valid_soap(xml: &str) -> Result<String> {
        let mut fixed = xml.to_string();

        // Ensure XML declaration is present
        if !fixed.starts_with("<?xml") {
            fixed = format!(r#"<?xml version="1.0" encoding="UTF-8"?>{}"#, fixed);
        }

        // Ensure SOAP envelope namespace is present
        if !fixed.contains("xmlns:SOAP-ENV=") {
            fixed = fixed.replace(
                "<SOAP-ENV:Envelope",
                r#"<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope""#,
            );
        }

        Ok(fixed)
    }
}
