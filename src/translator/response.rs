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
}
