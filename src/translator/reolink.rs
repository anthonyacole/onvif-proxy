use anyhow::Result;

pub struct ReolinkEventTranslator;

impl ReolinkEventTranslator {
    pub fn translate_response(xml: &str, quirks: &[String]) -> Result<String> {
        let mut result = xml.to_string();

        for quirk in quirks {
            result = match quirk.as_str() {
                "fix_device_info_namespace" => Self::fix_device_info_namespace(&result),
                "normalize_media_profiles" => Self::normalize_media_profiles(&result),
                "translate_smart_events" => Self::translate_smart_events(&result),
                "add_missing_namespaces" => Self::add_missing_namespaces(&result),
                _ => {
                    tracing::warn!("Unknown quirk: {}", quirk);
                    result
                }
            };
        }

        Ok(result)
    }

    fn fix_device_info_namespace(xml: &str) -> String {
        let mut fixed = xml.to_string();

        // Add xmlns:tds if missing
        if !fixed.contains("xmlns:tds=") && fixed.contains("<tds:") {
            fixed = Self::add_namespace(&fixed, "tds", "http://www.onvif.org/ver10/device/wsdl");
        }

        // Add xmlns:tt if missing
        if !fixed.contains("xmlns:tt=") && fixed.contains("<tt:") {
            fixed = Self::add_namespace(&fixed, "tt", "http://www.onvif.org/ver10/schema");
        }

        fixed
    }

    fn normalize_media_profiles(xml: &str) -> String {
        let mut fixed = xml.to_string();

        // Add xmlns:trt if missing
        if !fixed.contains("xmlns:trt=") && fixed.contains("<trt:") {
            fixed = Self::add_namespace(&fixed, "trt", "http://www.onvif.org/ver10/media/wsdl");
        }

        // Add xmlns:tt if missing
        if !fixed.contains("xmlns:tt=") && fixed.contains("<tt:") {
            fixed = Self::add_namespace(&fixed, "tt", "http://www.onvif.org/ver10/schema");
        }

        fixed
    }

    fn translate_smart_events(xml: &str) -> String {
        let mut fixed = xml.to_string();

        // Reolink smart detection event mappings
        let event_mappings = vec![
            // Person detection
            ("PeopleDetect", "Motion"),
            ("PersonDetection", "Motion"),
            ("tns1:RuleEngine/MyRuleDetector/PeopleDetect", "tns1:RuleEngine/CellMotionDetector/Motion"),

            // Vehicle detection
            ("VehicleDetect", "Motion"),
            ("VehicleDetection", "Motion"),
            ("tns1:RuleEngine/MyRuleDetector/VehicleDetect", "tns1:RuleEngine/CellMotionDetector/Motion"),

            // Pet/Animal detection
            ("DogCatDetect", "Motion"),
            ("PetDetection", "Motion"),
            ("tns1:RuleEngine/MyRuleDetector/DogCatDetect", "tns1:RuleEngine/CellMotionDetector/Motion"),

            // Face detection
            ("FaceDetect", "Motion"),
            ("FaceDetection", "Motion"),

            // Generic smart detection
            ("SmartDetection", "Motion"),
            ("AIDetection", "Motion"),
        ];

        for (reolink_event, onvif_event) in event_mappings {
            fixed = fixed.replace(reolink_event, onvif_event);
        }

        // Ensure tns1 namespace is present for topics
        if !fixed.contains("xmlns:tns1=") && fixed.contains("tns1:") {
            fixed = Self::add_namespace(&fixed, "tns1", "http://www.onvif.org/ver10/topics");
        }

        // Add missing event data fields that iSpy expects
        if fixed.contains("Motion") && !fixed.contains("State") {
            // Add State SimpleItem if motion event exists but State is missing
            if fixed.contains("<SimpleItem") && !fixed.contains(r#"Name="State""#) {
                fixed = fixed.replace(
                    "</wsnt:Message>",
                    r#"<tt:SimpleItem Name="State" Value="true"/></tt:Data></tt:Message></wsnt:Message>"#,
                );
            }
        }

        fixed
    }

    fn add_missing_namespaces(xml: &str) -> String {
        let mut fixed = xml.to_string();

        let namespaces = vec![
            ("tds", "http://www.onvif.org/ver10/device/wsdl"),
            ("trt", "http://www.onvif.org/ver10/media/wsdl"),
            ("tev", "http://www.onvif.org/ver10/events/wsdl"),
            ("tt", "http://www.onvif.org/ver10/schema"),
            ("tns1", "http://www.onvif.org/ver10/topics"),
            ("wsnt", "http://docs.oasis-open.org/wsn/b-2"),
        ];

        for (prefix, uri) in namespaces {
            let xmlns = format!("xmlns:{}=", prefix);
            let tag_prefix = format!("<{}:", prefix);

            if !fixed.contains(&xmlns) && fixed.contains(&tag_prefix) {
                fixed = Self::add_namespace(&fixed, prefix, uri);
            }
        }

        fixed
    }

    fn add_namespace(xml: &str, prefix: &str, uri: &str) -> String {
        // Add namespace declaration to the SOAP Envelope
        let namespace_decl = format!(r#" xmlns:{}="{}""#, prefix, uri);

        if let Some(pos) = xml.find("<SOAP-ENV:Envelope") {
            if let Some(end_pos) = xml[pos..].find('>') {
                let insert_pos = pos + end_pos;
                let before = &xml[..insert_pos];
                let after = &xml[insert_pos..];
                return format!("{}{}{}", before, namespace_decl, after);
            }
        }

        // Fallback: just return original if we can't find the envelope
        xml.to_string()
    }

    pub fn fix_namespace_errors(xml: &str) -> String {
        // This is a catch-all function to fix common namespace errors
        let mut fixed = Self::add_missing_namespaces(xml);

        // Remove duplicate namespace declarations (can happen after multiple fixes)
        fixed = Self::remove_duplicate_namespaces(&fixed);

        fixed
    }

    fn remove_duplicate_namespaces(xml: &str) -> String {
        let mut result = xml.to_string();
        let mut _seen_namespaces: std::collections::HashSet<String> = std::collections::HashSet::new();

        // This is a simplified approach - in production you'd use proper XML parsing
        let namespaces = vec!["tds", "trt", "tev", "tt", "tns1", "wsnt", "SOAP-ENV"];

        for ns in namespaces {
            let xmlns_pattern = format!("xmlns:{}=", ns);
            let count = result.matches(&xmlns_pattern).count();

            if count > 1 {
                // Keep only the first occurrence
                let mut first = true;
                while let Some(pos) = result.find(&xmlns_pattern) {
                    if first {
                        first = false;
                        // Find the end of this namespace declaration
                        if let Some(quote_end) = result[pos..].find('"') {
                            if let Some(second_quote) = result[pos + quote_end + 1..].find('"') {
                                let skip_pos = pos + quote_end + second_quote + 2;
                                result = format!("{}{}", &result[..skip_pos], &result[skip_pos..]);
                                continue;
                            }
                        }
                    } else {
                        // Remove duplicate
                        if let Some(quote_end) = result[pos..].find('"') {
                            if let Some(second_quote) = result[pos + quote_end + 1..].find('"') {
                                let end_pos = pos + quote_end + second_quote + 2;
                                result = format!("{}{}", &result[..pos], &result[end_pos..].trim_start());
                                continue;
                            }
                        }
                    }
                    break;
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_smart_events() {
        let xml = r#"<tns1:RuleEngine/MyRuleDetector/PeopleDetect>"#;
        let result = ReolinkEventTranslator::translate_smart_events(xml);
        assert!(result.contains("CellMotionDetector/Motion"));
    }

    #[test]
    fn test_add_namespace() {
        let xml = r#"<SOAP-ENV:Envelope><tt:Something/></SOAP-ENV:Envelope>"#;
        let result = ReolinkEventTranslator::fix_device_info_namespace(xml);
        assert!(result.contains(r#"xmlns:tt="http://www.onvif.org/ver10/schema""#));
    }
}
