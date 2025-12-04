use std::str::Lines;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SectionParseError {
    #[error("missing section header")]
    MissingSectionHeader,
    #[error("invalid section header")]
    InvalidHeader,
    #[error("invalid member syntax")]
    InvalidMemberSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionMember {
    pub section_name: String,
    pub member_name: String,
    pub expression_m: String,
    pub is_shared: bool,
}

pub fn parse_section_members(source: &str) -> Result<Vec<SectionMember>, SectionParseError> {
    let source = strip_leading_bom(source);
    let mut lines = source.lines();
    let section_name = find_section_name(&mut lines)?;

    let mut members = Vec::new();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if !trimmed.starts_with("shared") {
            continue;
        }

        let member = parse_shared_member(trimmed, &mut lines, &section_name)
            .ok_or(SectionParseError::InvalidMemberSyntax)?;
        members.push(member);
    }

    Ok(members)
}

fn find_section_name(lines: &mut Lines<'_>) -> Result<String, SectionParseError> {
    for line in lines.by_ref() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        match try_parse_section_header(trimmed) {
            Ok(Some(name)) => return Ok(name),
            Ok(None) => continue,
            Err(err) => return Err(err),
        }
    }

    Err(SectionParseError::MissingSectionHeader)
}

fn try_parse_section_header(line: &str) -> Result<Option<String>, SectionParseError> {
    let Some(rest) = line.strip_prefix("section") else {
        return Ok(None);
    };

    if !rest.starts_with(char::is_whitespace) && !rest.is_empty() {
        return Err(SectionParseError::InvalidHeader);
    }

    let header_body = rest.trim_start();
    if !header_body.ends_with(';') {
        return Err(SectionParseError::InvalidHeader);
    }

    let without_semicolon = &header_body[..header_body.len() - 1];
    let name_candidate = without_semicolon.trim();
    if name_candidate.is_empty() {
        return Err(SectionParseError::InvalidHeader);
    }

    let mut parts = name_candidate.split_whitespace();
    let name = parts.next().ok_or(SectionParseError::InvalidHeader)?;
    if parts.next().is_some() {
        return Err(SectionParseError::InvalidHeader);
    }

    if !is_valid_identifier(name) {
        return Err(SectionParseError::InvalidHeader);
    }

    Ok(Some(name.to_string()))
}

fn parse_shared_member(
    line: &str,
    remaining_lines: &mut Lines<'_>,
    section_name: &str,
) -> Option<SectionMember> {
    let rest = line.strip_prefix("shared")?;
    if !rest.starts_with(char::is_whitespace) && !rest.is_empty() {
        return None;
    }

    let body = rest.trim_start();
    if body.is_empty() {
        return None;
    }

    let (member_name, after_name) = parse_identifier(body)?;

    let mut expression_source = after_name;
    let eq_index = expression_source.find('=')?;
    if !expression_source[..eq_index].trim().is_empty() {
        return None;
    }
    expression_source = &expression_source[eq_index + 1..];

    let mut expression = expression_source.to_string();
    if let Some(idx) = expression_source.find(';') {
        expression.truncate(idx);
    } else {
        let mut terminator_index = None;
        while terminator_index.is_none() {
            let Some(next_line) = remaining_lines.next() else {
                break;
            };

            expression.push('\n');
            let offset = expression.len();
            expression.push_str(next_line);
            if let Some(idx) = next_line.find(';') {
                terminator_index = Some(offset + idx);
            }
        }

        if let Some(idx) = terminator_index {
            expression.truncate(idx);
        } else {
            return None;
        }
    }

    let expression_m = expression.trim().to_string();

    Some(SectionMember {
        section_name: section_name.to_string(),
        member_name: member_name.to_string(),
        expression_m,
        is_shared: true,
    })
}

fn parse_identifier(text: &str) -> Option<(String, &str)> {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with("#\"") {
        return parse_quoted_identifier(trimmed);
    }

    parse_unquoted_identifier(trimmed)
}

fn parse_unquoted_identifier(text: &str) -> Option<(String, &str)> {
    if text.is_empty() {
        return None;
    }

    let mut end = 0;
    for ch in text.chars() {
        if ch.is_whitespace() || ch == '=' {
            break;
        }
        end += ch.len_utf8();
    }

    if end == 0 {
        return None;
    }

    let (name, rest) = text.split_at(end);
    if !is_valid_identifier(name) {
        return None;
    }

    Some((name.to_string(), rest))
}

fn parse_quoted_identifier(text: &str) -> Option<(String, &str)> {
    let mut chars = text.char_indices();
    let (_, hash) = chars.next()?;
    if hash != '#' {
        return None;
    }
    if !matches!(chars.next(), Some((_, '"'))) {
        return None;
    }

    let mut name = String::new();
    while let Some((idx, ch)) = chars.next() {
        if ch == '"' {
            if let Some((_, next_ch)) = chars.clone().next()
                && next_ch == '"'
            {
                name.push('"');
                chars.next();
                continue;
            }
            let rest_start = idx + 1;
            let rest = &text[rest_start..];
            return Some((name, rest));
        }

        name.push(ch);
    }

    None
}

fn is_valid_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn strip_leading_bom(text: &str) -> &str {
    text.strip_prefix('\u{FEFF}').unwrap_or(text)
}
