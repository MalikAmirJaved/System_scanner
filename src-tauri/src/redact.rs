//! Best-effort redaction for text that leaves the process.
//!
//! This module is platform-neutral and has no I/O. It is used for two things:
//! error details written to the local log, and the support bundle preview shown
//! to the user. Redaction is deliberately conservative about what it recognizes
//! (paths, e-mail addresses, IPv4/IPv6, MAC addresses, SIDs, and known account
//! or machine names); anything it cannot recognize is left alone and must not
//! be logged in the first place.

/// Placeholder for a path that does not belong to the current user profile.
pub const PATH: &str = "<path>";
/// Placeholder for a UNC path, which embeds a server and share name.
pub const UNC_PATH: &str = "<unc-path>";
/// Placeholder for the current user's profile directory.
pub const USER_PROFILE: &str = "%USERPROFILE%";

/// A literal value to remove, with the placeholder that replaces it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secret {
    pub value: String,
    pub placeholder: &'static str,
}

impl Secret {
    pub fn new(value: impl Into<String>, placeholder: &'static str) -> Self {
        Self {
            value: value.into(),
            placeholder,
        }
    }
}

/// Applies literal secrets and shape-based rules to diagnostic text.
#[derive(Debug, Clone, Default)]
pub struct Redactor {
    secrets: Vec<Secret>,
}

impl Redactor {
    /// Builds a redactor from account and machine names.
    ///
    /// Values shorter than two characters are ignored because replacing them
    /// would destroy unrelated text. Duplicates are matched once, longest first.
    pub fn new(secrets: impl IntoIterator<Item = Secret>) -> Self {
        let mut kept: Vec<Secret> = Vec::new();

        for secret in secrets {
            let value = secret.value.trim().to_owned();
            if value.chars().count() < 2 {
                continue;
            }
            if kept
                .iter()
                .any(|existing| existing.value.eq_ignore_ascii_case(&value))
            {
                continue;
            }
            kept.push(Secret {
                value,
                placeholder: secret.placeholder,
            });
        }

        kept.sort_by_key(|secret| std::cmp::Reverse(secret.value.len()));
        Self { secrets: kept }
    }

    /// Returns the redacted text and how many replacements were made.
    pub fn apply(&self, input: &str) -> Redaction {
        let mut text = input.to_owned();
        let mut replacements = 0;

        for secret in &self.secrets {
            let (next, count) = replace_ignore_ascii_case(&text, &secret.value, secret.placeholder);
            text = next;
            replacements += count;
        }

        let (text, token_replacements) = redact_words(&text);
        Redaction {
            text,
            replacements: replacements + token_replacements,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redaction {
    pub text: String,
    pub replacements: usize,
}

/// Redacts shape-based tokens without knowing any account or machine names.
pub fn redact_text(input: &str) -> String {
    Redactor::default().apply(input).text
}

fn replace_ignore_ascii_case(haystack: &str, needle: &str, placeholder: &str) -> (String, usize) {
    if needle.is_empty() {
        return (haystack.to_owned(), 0);
    }

    let bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    let mut output = String::with_capacity(haystack.len());
    let mut last = 0;
    let mut index = 0;
    let mut count = 0;

    while index + needle_bytes.len() <= bytes.len() {
        let end = index + needle_bytes.len();
        let is_match = haystack.is_char_boundary(index)
            && haystack.is_char_boundary(end)
            && is_word_boundary(haystack, index)
            && is_word_end(haystack, end)
            && bytes[index..end].eq_ignore_ascii_case(needle_bytes);

        if is_match {
            output.push_str(&haystack[last..index]);
            output.push_str(placeholder);
            index = end;
            last = end;
            count += 1;
        } else {
            index += 1;
        }
    }

    if count == 0 {
        return (haystack.to_owned(), 0);
    }

    output.push_str(&haystack[last..]);
    (output, count)
}

/// A literal secret only matches when it stands on its own, so a short account
/// name cannot rewrite an unrelated word that happens to contain it.
fn is_word_boundary(haystack: &str, index: usize) -> bool {
    haystack[..index]
        .chars()
        .next_back()
        .is_none_or(|character| !character.is_alphanumeric())
}

fn is_word_end(haystack: &str, index: usize) -> bool {
    haystack[index..]
        .chars()
        .next()
        .is_none_or(|character| !character.is_alphanumeric())
}

fn redact_words(input: &str) -> (String, usize) {
    let mut output = String::with_capacity(input.len());
    let mut count = 0;
    let mut word_start = 0;
    let mut has_word = false;
    let mut quote: Option<char> = None;

    for (index, character) in input.char_indices() {
        if let Some(active) = quote {
            if character == active {
                quote = None;
            }
            continue;
        }

        match character {
            // Double quotes delimit `tracing` string fields; an apostrophe in
            // prose must not swallow the rest of the line as one token.
            '"' => {
                quote = Some(character);
                has_word = true;
            }
            character if character.is_whitespace() => {
                if has_word {
                    let (word, replaced) = redact_word(&input[word_start..index]);
                    output.push_str(&word);
                    count += replaced;
                }
                output.push(character);
                word_start = index + character.len_utf8();
                has_word = false;
            }
            _ => has_word = true,
        }
    }

    if has_word {
        let (word, replaced) = redact_word(&input[word_start..]);
        output.push_str(&word);
        count += replaced;
    }

    (output, count)
}

fn redact_word(word: &str) -> (String, usize) {
    let (prefix, core, suffix) = split_punctuation(word);

    if is_email(core) {
        return (format!("{prefix}<email>{suffix}"), 1);
    }
    if is_sid(core) {
        return (format!("{prefix}<sid>{suffix}"), 1);
    }
    if is_ipv4(core) {
        return (format!("{prefix}<ip>{suffix}"), 1);
    }
    if let Some((host, port)) = core.rsplit_once(':')
        && is_ipv4(host)
        && !port.is_empty()
        && port.len() <= 5
        && port.chars().all(|character| character.is_ascii_digit())
    {
        return (format!("{prefix}<ip>:<port>{suffix}"), 1);
    }
    if is_mac(core) {
        return (format!("{prefix}<mac>{suffix}"), 1);
    }
    if is_ipv6(core) {
        return (format!("{prefix}<ip>{suffix}"), 1);
    }

    let Some(start) = path_start(core) else {
        return (word.to_owned(), 0);
    };

    let path = &core[start..];
    let replacement = redact_path(path);

    (
        format!("{prefix}{}{replacement}{suffix}", &core[..start]),
        1,
    )
}

fn redact_path(path: &str) -> String {
    if path.starts_with(r"\\") {
        return UNC_PATH.to_owned();
    }

    match after_user_segment(path) {
        Some(rest) => format!("{USER_PROFILE}\\{rest}"),
        None => PATH.to_owned(),
    }
}

/// Splits a token into leading and trailing punctuation and the part worth
/// inspecting. Quotes count as punctuation so `path="C:\x"` keeps its syntax.
fn split_punctuation(word: &str) -> (&str, &str, &str) {
    const EDGES: &[char] = &[
        '"', '\'', '`', '(', ')', '[', ']', '{', '}', '<', '>', ',', ';', '!', '?', '.',
    ];

    let start = word
        .char_indices()
        .find(|(_, character)| !EDGES.contains(character))
        .map_or(word.len(), |(index, _)| index);

    let end = word
        .char_indices()
        .rev()
        .find(|(_, character)| !EDGES.contains(character))
        .map_or(start, |(index, character)| index + character.len_utf8());

    (&word[..start], &word[start..end], &word[end..])
}

fn path_start(token: &str) -> Option<usize> {
    let bytes = token.as_bytes();

    (0..bytes.len()).find(|&index| {
        let drive = index + 2 < bytes.len()
            && bytes[index].is_ascii_alphabetic()
            && bytes[index + 1] == b':'
            && matches!(bytes[index + 2], b'\\' | b'/')
            && (index == 0 || !bytes[index - 1].is_ascii_alphanumeric());
        let unc = index + 1 < bytes.len() && bytes[index] == b'\\' && bytes[index + 1] == b'\\';

        drive || unc
    })
}

fn after_user_segment(token: &str) -> Option<&str> {
    let lower = token.to_ascii_lowercase();
    let marker = lower.find(r"\users\")?;
    let rest = &token[marker + r"\users\".len()..];
    let separator = rest.find(['\\', '/'])?;
    Some(&rest[separator + 1..])
}

fn is_email(token: &str) -> bool {
    let Some((local, domain)) = token.split_once('@') else {
        return false;
    };

    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && domain
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
}

fn is_sid(token: &str) -> bool {
    token.strip_prefix("S-1-").is_some_and(|rest| {
        rest.chars()
            .all(|character| character.is_ascii_digit() || character == '-')
    })
}

fn is_ipv4(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    parts.len() == 4
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.len() <= 3
                && part.chars().all(|character| character.is_ascii_digit())
                && part.parse::<u16>().is_ok_and(|value| value <= 255)
        })
}

fn is_mac(token: &str) -> bool {
    let normalised = token.replace('-', ":");
    let parts: Vec<&str> = normalised.split(':').collect();
    parts.len() == 6
        && parts.iter().all(|part| {
            part.len() == 2 && part.chars().all(|character| character.is_ascii_hexdigit())
        })
}

fn is_ipv6(token: &str) -> bool {
    if !token.contains(':') {
        return false;
    }

    // A compressed address always has `::`; a full one has at least three
    // colons. Both forms are made only of hex digits, colons, and an optional
    // `%zone` suffix.
    let compressed = token.contains("::");
    let colon_count = token.matches(':').count();

    if !compressed && colon_count < 3 {
        return false;
    }

    let address = token.split('%').next().unwrap_or(token);
    address
        .chars()
        .all(|character| character.is_ascii_hexdigit() || character == ':')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_paths_keep_their_suffix_and_other_paths_are_replaced() {
        assert_eq!(
            redact_text("failed to open C:\\Users\\ada\\AppData\\Local\\x.txt"),
            "failed to open %USERPROFILE%\\AppData\\Local\\x.txt"
        );
        assert_eq!(
            redact_text("scanned D:\\data\\secret.txt"),
            "scanned <path>"
        );
        assert_eq!(redact_text("no path here"), "no path here");
    }

    #[test]
    fn quoted_and_keyed_paths_keep_their_syntax() {
        assert_eq!(
            redact_text("detail=\"open C:\\Users\\ada\\report.txt failed\""),
            "detail=\"open %USERPROFILE%\\report.txt failed\""
        );
        assert_eq!(
            redact_text(r"share=\\server\team\plans\q4.xlsx"),
            "share=<unc-path>"
        );
    }

    #[test]
    fn spaces_inside_a_quoted_path_do_not_split_the_redaction() {
        assert_eq!(
            redact_text(r#"detail="failed to open C:\Users\ada\My Documents\secret.txt" after"#),
            r#"detail="failed to open %USERPROFILE%\My Documents\secret.txt" after"#
        );
    }

    #[test]
    fn shape_based_tokens_are_redacted() {
        assert_eq!(
            redact_text("ada@example.com from 192.0.2.10 mac 00:1A:2B:3C:4D:5E sid S-1-5-21-1-2-3"),
            "<email> from <ip> mac <mac> sid <sid>"
        );
        assert_eq!(
            redact_text("listening on fe80::1 and 2001:db8::10"),
            "listening on <ip> and <ip>"
        );
        assert_eq!(
            redact_text("connected to 192.0.2.10:443"),
            "connected to <ip>:<port>"
        );
    }

    #[test]
    fn account_and_machine_names_are_removed_case_insensitively() {
        let redactor = Redactor::new([
            Secret::new("Ada", "<user>"),
            Secret::new("DESKTOP-ABC", "<host>"),
        ]);

        let redaction = redactor.apply("user ada on desktop-abc opened C:\\Users\\Ada\\x.txt");

        assert_eq!(
            redaction.text,
            "user <user> on <host> opened %USERPROFILE%\\x.txt"
        );
        assert_eq!(redaction.replacements, 4);
    }

    #[test]
    fn whitespace_and_newlines_are_preserved() {
        let text = "first line\n\tsecond  line\n";

        assert_eq!(redact_text(text), text);
    }
}
