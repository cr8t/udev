use crate::{Error, Result};

/// Gets whether the provided character is whitelisted.
pub fn whitelisted_char_for_devnode(c: char, white: &str) -> bool {
    c.is_ascii_digit()
        || c.is_ascii_uppercase()
        || c.is_ascii_lowercase()
        || "#+-.:=@_".contains(c)
        || white.contains(c)
}

/// Encodes a `devnode` name, removing potentially dangerous characters.
pub fn encode_devnode_name(arg: &str) -> Result<String> {
    if arg.is_empty() {
        Err(Error::UdevUtil("empty encode string".into()))
    } else {
        let arg_len = arg.len();
        let mut ret = String::with_capacity(arg_len.saturating_mul(4));
        // check for a nul-terminated string
        let null_pos = arg.find('\0').unwrap_or(arg_len);

        for c in arg[..null_pos].chars() {
            let seqlen = c.len_utf8();
            if seqlen > 1 {
                let mut bytes = [0u8; 4];
                ret.push_str(c.encode_utf8(&mut bytes));
            } else if c == '\\' || !whitelisted_char_for_devnode(c, "") {
                ret = format!("{ret}\\x{:02x}", c as u8);
            } else {
                ret.push(c);
            }
        }

        Ok(ret)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encode_devnode_string_empty() {
        assert!(encode_devnode_name("").is_err());
    }

    #[test]
    fn test_whitelisted_char_for_devnode() {
        // check ASCII digits are whitelisted
        for c in '0'..='9' {
            assert!(whitelisted_char_for_devnode(c, ""));
        }
        // check ASCII lowercase are whitelisted
        for c in 'a'..='z' {
            assert!(whitelisted_char_for_devnode(c, ""));
        }
        // check ASCII uppercase are whitelisted
        for c in 'A'..='Z' {
            assert!(whitelisted_char_for_devnode(c, ""));
        }
        // check ASCII special are whitelisted
        for c in "#+-.:=@_".chars() {
            assert!(whitelisted_char_for_devnode(c, ""));
        }
        // check non-default whitelist are rejected
        for c in "`~%^&*(){}!$|\\".chars() {
            assert!(!whitelisted_char_for_devnode(c, ""));
        }
        // check non-default whitelist accepted with custom whitelist
        for c in "`~%^&*(){}!$|\\".chars() {
            assert!(whitelisted_char_for_devnode(c, "`~%^&*(){}!$|\\"));
        }
    }

    #[test]
    fn test_encode_devnode_name() -> Result<()> {
        let arg_str: String = "#+-.:=@_"
            .chars()
            .chain(('a'..='z').chain('A'..='Z').chain('0'..='9'))
            .collect();
        let enc_str = encode_devnode_name(arg_str.as_str())?;

        assert_eq!(arg_str.as_str(), enc_str.as_str());

        let esc_str = "`~%^&*(){}!$|\\";
        let exp_str = encode_devnode_name(esc_str)?;

        // check non-whitelisted ASCII characters
        assert_eq!(esc_str.len().saturating_mul(4), exp_str.len());
        assert_eq!(
            exp_str.as_str(),
            "\\x60\\x7e\\x25\\x5e\\x26\\x2a\\x28\\x29\\x7b\\x7d\\x21\\x24\\x7c\\x5c"
        );

        // check multi-byte UTF-8 characters
        let utf8_str = "💖";
        let exp_utf8_str = encode_devnode_name(utf8_str)?;
        assert_eq!(exp_utf8_str.as_str(), "💖");

        Ok(())
    }
}
