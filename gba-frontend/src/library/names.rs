//! Display names from file names (`docs/gui/SCREENS.md`, 01 Library).
//!
//! `Kingdom Hearts - Chain of Memories (USA).zip` becomes title `Kingdom Hearts` and subtitle
//! `Chain of Memories · USA`.
//!
//! Rules, in order:
//! 1. The extension (`.zip`, `.gba`, `.nes`: the last dot and 1-4 letters or digits) is dropped.
//! 2. Every parenthesised tag is lifted out of the name. A language list such as `(En,Fr,De,It)`
//!    and a revision such as `(Rev 1)` are dropped; the rest (`USA`, `USA, Europe`, `Japan`,
//!    `Beta`) become the trailing parts of the subtitle.
//! 3. A trailing `, The` / `, A` / `, An` moves to the front of its part
//!    (`Legend of Zelda, The` -> `The Legend of Zelda`).
//! 4. The remainder is split on ` - `.
//!    * one part: the title alone;
//!    * two parts: title and subtitle;
//!    * three or more parts: the first part is a series prefix. The second part is the title, the
//!      rest is the subtitle, and the prefix goes after the subtitle
//!      (`Classic NES Series - Zelda II - The Adventure of Link (USA, Europe)` -> title
//!      `Zelda II`, subtitle `The Adventure of Link · Classic NES Series · USA, Europe`).
//!    * a two-part name whose first part is a known series prefix (`Classic NES Series`,
//!      `Famicom Mini`) is treated the same way: `Classic NES Series - Legend of Zelda, The`
//!      -> title `The Legend of Zelda`, subtitle `Classic NES Series · USA, Europe`.
//! 5. Subtitle parts are joined with ` · `.
//!
//! When the file name has nothing usable the caller falls back to the cartridge header title.

/// Series prefixes that make a two-part name read `<series> - <game>`.
const SERIES_PREFIXES: [&str; 3] = ["Classic NES Series", "Famicom Mini", "Game Boy Advance Video"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayName {
    pub title: String,
    pub subtitle: String,
}

/// Display name for a file name (with or without directory), or None when the stem is empty.
pub fn from_file_name(file_name: &str) -> Option<DisplayName> {
    let base = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let stem = strip_extension(base).trim();
    if stem.is_empty() {
        return None;
    }
    let (body, tags) = split_tags(stem);
    let parts: Vec<String> = body.split(" - ").map(|p| front_article(p.trim())).filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut sub: Vec<String> = Vec::new();
    let title;
    let has_series = parts.len() >= 3 || (parts.len() == 2 && SERIES_PREFIXES.iter().any(|s| s.eq_ignore_ascii_case(&parts[0])));
    if has_series {
        title = parts[1].clone();
        sub.extend(parts[2..].iter().cloned());
        sub.push(parts[0].clone());
    } else {
        title = parts[0].clone();
        sub.extend(parts[1..].iter().cloned());
    }
    sub.extend(tags);
    Some(DisplayName { title, subtitle: sub.join(" · ") })
}

/// Name from the 12-character header title when the file name is useless: `KINGDOM HEART`
/// stays upper case (the screens draw titles upper case anyway).
pub fn from_header_title(header_title: &str) -> DisplayName {
    DisplayName { title: header_title.trim().to_string(), subtitle: String::new() }
}

fn strip_extension(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((stem, ext)) if (1..=4).contains(&ext.len()) && ext.bytes().all(|b| b.is_ascii_alphanumeric()) => stem,
        _ => name,
    }
}

/// Remove every `( ... )` group; return the text without them and the kept tags in order.
fn split_tags(s: &str) -> (String, Vec<String>) {
    let mut body = String::with_capacity(s.len());
    let mut tags = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '(' => {
                depth += 1;
                if depth > 1 {
                    cur.push(ch);
                }
            }
            ')' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    let t = cur.trim().to_string();
                    cur.clear();
                    if !t.is_empty() && !is_dropped_tag(&t) {
                        tags.push(t);
                    }
                } else {
                    cur.push(ch);
                }
            }
            _ if depth > 0 => cur.push(ch),
            _ => body.push(ch),
        }
    }
    // an unclosed "(" keeps its text in the name
    if depth > 0 {
        body.push('(');
        body.push_str(&cur);
    }
    let body = body.split_whitespace().collect::<Vec<_>>().join(" ");
    (body, tags)
}

/// `En,Fr,De,It` (two-letter language codes) and `Rev 1` / `v1.1` carry no information a player
/// needs on a cartridge label.
fn is_dropped_tag(tag: &str) -> bool {
    let is_lang = |p: &str| {
        let mut c = p.trim().chars();
        matches!((c.next(), c.next(), c.next()), (Some(a), Some(b), None) if a.is_ascii_uppercase() && b.is_ascii_lowercase())
    };
    if tag.split(',').all(is_lang) {
        return true;
    }
    let lower = tag.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("rev") {
        return rest.trim().chars().all(|c| c.is_ascii_alphanumeric() || c == '.');
    }
    if let Some(rest) = lower.strip_prefix('v') {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '.');
    }
    false
}

/// `Legend of Zelda, The` -> `The Legend of Zelda`.
fn front_article(part: &str) -> String {
    for art in ["The", "A", "An"] {
        let suffix = format!(", {art}");
        if let Some(head) = part.strip_suffix(&suffix) {
            return format!("{art} {head}");
        }
    }
    part.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(file: &str) -> (String, String) {
        let d = from_file_name(file).expect(file);
        (d.title, d.subtitle)
    }

    #[test]
    fn real_file_names() {
        let cases = [
            ("Kingdom Hearts - Chain of Memories (USA).zip", "Kingdom Hearts", "Chain of Memories · USA"),
            ("Legend of Zelda, The - The Minish Cap (USA).zip", "The Legend of Zelda", "The Minish Cap · USA"),
            ("Legend of Zelda, The - A Link to the Past & Four Swords (USA).zip", "The Legend of Zelda", "A Link to the Past & Four Swords · USA"),
            ("Classic NES Series - Legend of Zelda, The (USA, Europe).zip", "The Legend of Zelda", "Classic NES Series · USA, Europe"),
            ("Classic NES Series - Zelda II - The Adventure of Link (USA, Europe).zip", "Zelda II", "The Adventure of Link · Classic NES Series · USA, Europe"),
            ("Initial D - Another Stage (Japan).zip", "Initial D", "Another Stage · Japan"),
            ("Mario Kart - Super Circuit (USA).zip", "Mario Kart", "Super Circuit · USA"),
            ("Final Fantasy VI Advance (USA).zip", "Final Fantasy VI Advance", "USA"),
            ("Need for Speed - Underground 2 (USA, Europe) (En,Fr,De,It).zip", "Need for Speed", "Underground 2 · USA, Europe"),
            ("Crash Bandicoot - The Huge Adventure (USA).zip", "Crash Bandicoot", "The Huge Adventure · USA"),
            ("Super Mario Advance 4 - Super Mario Bros. 3 (USA, Australia) (Rev 1).zip", "Super Mario Advance 4", "Super Mario Bros. 3 · USA, Australia"),
        ];
        for (file, title, sub) in cases {
            assert_eq!(n(file), (title.to_string(), sub.to_string()), "{file}");
        }
    }

    #[test]
    fn extension_directory_and_case() {
        assert_eq!(n("C:\\Games\\Mario Kart - Super Circuit (USA).GBA"), ("Mario Kart".into(), "Super Circuit · USA".into()));
        assert_eq!(n("/roms/Plain Game.gba"), ("Plain Game".into(), String::new()));
        assert_eq!(n("Super Mario Bros. 3 (USA).nes"), ("Super Mario Bros. 3".into(), "USA".into()));
        assert_eq!(n("Dr. Mario"), ("Dr. Mario".into(), String::new()), "no extension: nothing dropped");
        assert_eq!(n("no extension (Japan)"), ("no extension".into(), "Japan".into()));
    }

    #[test]
    fn empty_names_fall_back() {
        assert!(from_file_name("").is_none());
        assert!(from_file_name(".zip").is_none());
        assert!(from_file_name("(USA).gba").is_none());
        assert_eq!(from_header_title("KINGDOM HEART ").title, "KINGDOM HEART");
    }

    #[test]
    fn tags_are_classified() {
        assert!(is_dropped_tag("En,Fr,De,It"));
        assert!(is_dropped_tag("En"));
        assert!(is_dropped_tag("Rev 1"));
        assert!(is_dropped_tag("v1.1"));
        assert!(!is_dropped_tag("USA, Europe"));
        assert!(!is_dropped_tag("Beta"));
        assert!(!is_dropped_tag("Japan"));
        assert_eq!(n("Game (Beta) (USA).gba").1, "Beta · USA");
    }

    #[test]
    fn unbalanced_parentheses_do_not_lose_text() {
        assert_eq!(n("Odd (name.gba").0, "Odd (name");
    }
}
