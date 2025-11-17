use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, Writer};
use std::io::Cursor;
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct SoapEnvelope {
    pub header: Option<SoapHeader>,
    pub body: SoapBody,
    pub namespaces: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct SoapHeader {
    pub security: Option<WsSecurity>,
    pub raw_xml: String,
}

#[derive(Debug, Clone)]
pub struct WsSecurity {
    pub username: String,
    pub password_digest: String,
    pub nonce: String,
    pub created: String,
}

#[derive(Debug, Clone)]
pub struct SoapBody {
    pub action: String,
    pub content: String,
    pub raw_xml: String,
}

impl SoapEnvelope {
    pub fn parse(xml: &str) -> Result<Self> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut namespaces = Vec::new();
        let mut header = None;
        let mut body = None;
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    if e.name().as_ref() == b"Envelope" {
                        // Extract namespaces from Envelope element
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let value = String::from_utf8_lossy(&attr.value).to_string();
                                if key.starts_with("xmlns") {
                                    namespaces.push((key, value));
                                }
                            }
                        }
                    } else if e.name().as_ref() == b"Header" {
                        header = Some(Self::parse_header(&mut reader)?);
                    } else if e.name().as_ref() == b"Body" {
                        body = Some(Self::parse_body(&mut reader)?);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(anyhow::anyhow!("XML parsing error: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(SoapEnvelope {
            header,
            body: body.context("SOAP Body not found")?,
            namespaces,
        })
    }

    fn parse_header(reader: &mut Reader<&[u8]>) -> Result<SoapHeader> {
        let mut raw_xml = String::new();
        let mut buf = Vec::new();
        let mut depth = 1;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    depth += 1;
                    raw_xml.push_str(&format!("<{}>", String::from_utf8_lossy(e.as_ref())));
                }
                Ok(Event::End(e)) => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    raw_xml.push_str(&format!("</{}>", String::from_utf8_lossy(e.as_ref())));
                }
                Ok(Event::Text(e)) => {
                    raw_xml.push_str(&e.unescape().unwrap_or_default());
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(anyhow::anyhow!("Header parsing error: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(SoapHeader {
            security: None,
            raw_xml,
        })
    }

    fn parse_body(reader: &mut Reader<&[u8]>) -> Result<SoapBody> {
        let mut raw_xml = String::new();
        let mut action = String::new();
        let mut content = String::new();
        let mut buf = Vec::new();
        let mut depth = 1;
        let mut capture_content = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    depth += 1;
                    let tag_name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();

                    if depth == 2 {
                        // This is the action element (first element in Body)
                        action = tag_name.clone();
                        capture_content = true;
                    }

                    raw_xml.push_str(&format!("<{}", tag_name));
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref());
                        let value = String::from_utf8_lossy(&attr.value);
                        raw_xml.push_str(&format!(" {}=\"{}\"", key, value));
                    }
                    raw_xml.push('>');
                }
                Ok(Event::End(e)) => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    let tag_name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                    raw_xml.push_str(&format!("</{}>", tag_name));
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape().unwrap_or_default();
                    raw_xml.push_str(&text);
                    if capture_content {
                        content.push_str(&text);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(anyhow::anyhow!("Body parsing error: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(SoapBody {
            action,
            content,
            raw_xml,
        })
    }

    pub fn create_response(action: String, body_content: String) -> Self {
        let namespaces = vec![
            ("xmlns:SOAP-ENV".to_string(), "http://www.w3.org/2003/05/soap-envelope".to_string()),
            ("xmlns:tds".to_string(), "http://www.onvif.org/ver10/device/wsdl".to_string()),
            ("xmlns:trt".to_string(), "http://www.onvif.org/ver10/media/wsdl".to_string()),
            ("xmlns:tev".to_string(), "http://www.onvif.org/ver10/events/wsdl".to_string()),
            ("xmlns:tt".to_string(), "http://www.onvif.org/ver10/schema".to_string()),
            ("xmlns:tns1".to_string(), "http://www.onvif.org/ver10/topics".to_string()),
        ];

        SoapEnvelope {
            header: None,
            body: SoapBody {
                action: format!("{}Response", action),
                content: body_content.clone(),
                raw_xml: format!("<{}Response>{}</{}Response>", action, body_content, action),
            },
            namespaces,
        }
    }

    pub fn to_xml(&self) -> Result<String> {
        let mut xml = String::new();
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push_str("<SOAP-ENV:Envelope");

        // Add namespaces
        for (key, value) in &self.namespaces {
            xml.push_str(&format!(r#" {}="{}""#, key, value));
        }
        xml.push('>');

        // Add header if present
        if let Some(header) = &self.header {
            xml.push_str("<SOAP-ENV:Header>");
            xml.push_str(&header.raw_xml);
            xml.push_str("</SOAP-ENV:Header>");
        }

        // Add body
        xml.push_str("<SOAP-ENV:Body>");
        xml.push_str(&self.body.raw_xml);
        xml.push_str("</SOAP-ENV:Body>");

        xml.push_str("</SOAP-ENV:Envelope>");

        Ok(xml)
    }

    pub fn extract_action(&self) -> String {
        self.body.action.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_soap() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope">
  <SOAP-ENV:Body>
    <GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>
  </SOAP-ENV:Body>
</SOAP-ENV:Envelope>"#;

        let envelope = SoapEnvelope::parse(xml).unwrap();
        assert_eq!(envelope.body.action, "GetDeviceInformation");
    }
}
