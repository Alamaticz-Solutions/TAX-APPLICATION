use crate::WorkdayProviderError;

pub(crate) fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(crate) fn validate_xml_value(
    name: &'static str,
    value: &str,
) -> Result<(), WorkdayProviderError> {
    if value.is_empty() {
        return Err(WorkdayProviderError::InvalidParameter {
            name,
            reason: "must not be empty",
        });
    }

    if value.chars().any(|ch| ch.is_control()) {
        return Err(WorkdayProviderError::InvalidParameter {
            name,
            reason: "must not contain control characters",
        });
    }

    Ok(())
}

pub(crate) fn validate_date_literal(
    name: &'static str,
    value: &str,
) -> Result<(), WorkdayProviderError> {
    let bytes = value.as_bytes();
    let has_date_shape = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit());

    if has_date_shape {
        return Ok(());
    }

    Err(WorkdayProviderError::InvalidParameter {
        name,
        reason: "must be an ISO-8601 date literal in YYYY-MM-DD form",
    })
}

pub(crate) fn validate_datetime_literal(
    name: &'static str,
    value: &str,
) -> Result<(), WorkdayProviderError> {
    let bytes = value.as_bytes();
    let has_utc_shape = bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
        && bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        });

    if has_utc_shape {
        return Ok(());
    }

    Err(WorkdayProviderError::InvalidParameter {
        name,
        reason: "must be a UTC ISO-8601 datetime literal in YYYY-MM-DDTHH:MM:SSZ form",
    })
}

pub(crate) fn extract_tag_text(xml: &str, tag_name: &str) -> Option<String> {
    let local_open = format!("<{tag_name}>");
    let local_close = format!("</{tag_name}>");

    if let Some(value) = extract_between(xml, &local_open, &local_close) {
        return Some(value);
    }

    let prefixed_open_suffix = format!(":{tag_name}>");
    let prefixed_close_suffix = format!(":{tag_name}>");
    let open_start = xml.find('<')?;
    let mut search_from = open_start;

    while let Some(relative_open) = xml[search_from..].find('<') {
        let open = search_from + relative_open;
        let after_open = &xml[open..];
        let open_end_relative = after_open.find('>')?;
        let open_end = open + open_end_relative + 1;
        let opening = &xml[open..open_end];

        if opening.ends_with(&prefixed_open_suffix) && !opening.starts_with("</") {
            let close_marker = format!("</{}", &opening[1..opening.len() - 1]);
            let close_marker = format!("{close_marker}>");
            return extract_between(&xml[open_end..], "", &close_marker);
        }

        if opening.ends_with(&prefixed_close_suffix) {
            return None;
        }

        search_from = open_end;
    }

    None
}

fn extract_between(xml: &str, open: &str, close: &str) -> Option<String> {
    let content_start = if open.is_empty() {
        0
    } else {
        xml.find(open)? + open.len()
    };
    let content_end = xml[content_start..].find(close)? + content_start;
    Some(xml[content_start..content_end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_xml_control_characters() {
        assert_eq!(
            escape_xml_text("A&B <C> \"D\" 'E'"),
            "A&amp;B &lt;C&gt; &quot;D&quot; &apos;E&apos;"
        );
    }

    #[test]
    fn extracts_prefixed_tag_text() {
        let xml = "<wd:Page>2</wd:Page>";

        assert_eq!(extract_tag_text(xml, "Page").as_deref(), Some("2"));
    }
}
