//! Session-cookie parsing from `.env` text, without disclosing the value in errors.

use std::fmt;

use zeroize::Zeroizing;

use crate::error::OpsError;

/// The `adventofcode.com` session cookie, wiped from memory on drop.
pub(crate) struct Session(Zeroizing<String>);

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Session([redacted])")
    }
}

impl Session {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Cookie octets per RFC 6265, which excludes quotes, commas, semicolons and backslashes.
const fn is_cookie_octet(byte: u8) -> bool {
    matches!(byte, 0x21 | 0x23..=0x2B | 0x2D..=0x3A | 0x3C..=0x5B | 0x5D..=0x7E)
}

/// Returns the first non-empty `AOC_SESSION` value, unquoted, as a validated cookie.
pub(crate) fn parse(text: &str) -> Result<Session, OpsError> {
    let value = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .filter(|(key, _)| key.trim() == "AOC_SESSION")
        .map(|(_, value)| {
            let value = value.trim();
            value
                .strip_prefix('"')
                .and_then(|inner| inner.strip_suffix('"'))
                .or_else(|| {
                    value
                        .strip_prefix('\'')
                        .and_then(|inner| inner.strip_suffix('\''))
                })
                .unwrap_or(value)
        })
        .find(|value| !value.trim().is_empty())
        .ok_or(OpsError::SessionMissing)?;
    if value.bytes().all(is_cookie_octet) {
        Ok(Session(Zeroizing::new(value.to_owned())))
    } else {
        Err(OpsError::SessionInvalid)
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::error::OpsError;

    #[test]
    fn reads_the_first_non_empty_value_and_strips_quotes() {
        let text = "# comment\nOTHER=x\nAOC_SESSION=\n AOC_SESSION = \"53ab\" \nAOC_SESSION=zz\n";
        assert_eq!(
            parse(text).unwrap().as_str(),
            "53ab",
            "first non-empty value wins"
        );
        assert_eq!(
            parse("AOC_SESSION='ab'").unwrap().as_str(),
            "ab",
            "single quotes"
        );
        assert_eq!(
            parse("AOC_SESSION=ab=c").unwrap().as_str(),
            "ab=c",
            "split once"
        );
    }

    #[test]
    fn missing_or_unsafe_values_are_rejected_without_their_contents() {
        assert!(
            matches!(parse("AOC_SESSION=\n"), Err(OpsError::SessionMissing)),
            "empty value"
        );
        assert!(
            matches!(parse("#AOC_SESSION=ab"), Err(OpsError::SessionMissing)),
            "commented out"
        );
        for bad in [
            "AOC_SESSION=x y",
            "AOC_SESSION=x;y",
            "AOC_SESSION=x\\y",
            "AOC_SESSION=xé",
        ] {
            let error = parse(bad).unwrap_err();
            assert!(
                matches!(error, OpsError::SessionInvalid),
                "{bad} is rejected"
            );
            assert!(!error.to_string().contains('x'), "{error} hides the value");
        }
    }

    #[test]
    fn debug_output_redacts_the_cookie() {
        let session = parse("AOC_SESSION=secret").unwrap();
        assert_eq!(
            format!("{session:?}"),
            "Session([redacted])",
            "value is hidden"
        );
    }
}
