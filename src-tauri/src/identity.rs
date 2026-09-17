//! Who is at the keyboard — for the Home greeting only.
//!
//! There is no account, so the name comes from the OS. On Windows the real
//! display name ("João Vieira") sits behind `GetUserNameExW`; the account
//! name (`joao_`) is what `%USERNAME%` gives. Anything that fails falls back
//! to a prettified account name, and `Config::display_name` overrides both.

/// `joao_` → `Joao`, `john.doe` → `John Doe`, `CISCO95` → `Cisco`.
///
/// Splits on the separators account names use, drops trailing digits from
/// each part, and title-cases what is left. Never panics, may return "".
pub fn prettify_username(raw: &str) -> String {
    raw.split(|c: char| c == '_' || c == '.' || c == '-' || c.is_whitespace())
        .map(|part| part.trim_end_matches(|c: char| c.is_ascii_digit()))
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first
                    .to_uppercase()
                    .chain(chars.flat_map(char::to_lowercase))
                    .collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// The OS display name, or a prettified account name when the OS has none.
pub fn os_display_name() -> String {
    windows_display_name()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| {
            let account = std::env::var("USERNAME")
                .or_else(|_| std::env::var("USER"))
                .unwrap_or_default();
            prettify_username(&account)
        })
}

/// `GetUserNameExW(NameDisplay)` from secur32, declared by hand: one
/// function does not justify another `windows` crate feature. Local
/// accounts without a full name fail with ERROR_NONE_MAPPED, which the
/// caller treats as "use the account name".
#[cfg(windows)]
fn windows_display_name() -> Option<String> {
    #[link(name = "secur32")]
    extern "system" {
        fn GetUserNameExW(name_format: i32, name_buffer: *mut u16, size: *mut u32) -> u8;
    }
    const NAME_DISPLAY: i32 = 3;
    let mut buf = [0u16; 256];
    let mut size = buf.len() as u32;
    // SAFETY: buf outlives the call and `size` is its capacity in u16s; the
    // function writes at most `size` units and updates `size` on return.
    let ok = unsafe { GetUserNameExW(NAME_DISPLAY, buf.as_mut_ptr(), &mut size) };
    if ok == 0 || size == 0 || size as usize > buf.len() {
        return None;
    }
    // On success `size` is the length without the trailing NUL.
    Some(String::from_utf16_lossy(&buf[..size as usize]))
}

#[cfg(not(windows))]
fn windows_display_name() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prettify_strips_the_trailing_underscore() {
        assert_eq!(prettify_username("joao_"), "Joao");
    }

    #[test]
    fn prettify_title_cases_each_part() {
        assert_eq!(prettify_username("john.doe"), "John Doe");
        assert_eq!(prettify_username("mary-ann smith"), "Mary Ann Smith");
    }

    #[test]
    fn prettify_drops_trailing_digits_and_shouting() {
        assert_eq!(prettify_username("CISCO95"), "Cisco");
        assert_eq!(prettify_username("user2024_"), "User");
    }

    #[test]
    fn prettify_keeps_non_ascii_letters() {
        assert_eq!(prettify_username("joão"), "João");
    }

    #[test]
    fn prettify_of_nothing_is_empty() {
        assert_eq!(prettify_username(""), "");
        assert_eq!(prettify_username("_._"), "");
        assert_eq!(prettify_username("42"), "");
    }

    #[test]
    fn os_display_name_does_not_panic() {
        // Environment-dependent value; the contract is only "a String".
        let _ = os_display_name();
    }
}
