pub(super) fn normalize_match_key(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    for ch in value.chars().flat_map(char::to_lowercase) {
        append_folded_char(ch, &mut normalized);
    }
    normalized
}

fn append_folded_char(ch: char, out: &mut String) {
    if is_combining_mark(ch) {
        return;
    }

    match ch {
        // Turkish dotless-i is folded to plain i for stable search behavior.
        '\u{0131}' => out.push('i'),
        // Fold sharp-s to "ss" for stable case-insensitive matching.
        '\u{00DF}' => out.push_str("ss"),
        // Common Latin ligatures that users expect to match by expanded form.
        '\u{00E6}' => out.push_str("ae"),
        '\u{0153}' => out.push_str("oe"),
        _ => {
            if let Some(base) = fold_precomposed_latin(ch) {
                out.push(base);
            } else {
                out.push(ch);
            }
        }
    }
}

fn fold_precomposed_latin(ch: char) -> Option<char> {
    Some(match ch {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' | 'ǎ' | 'ȁ' | 'ȃ' => 'a',
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => 'c',
        'ď' | 'đ' => 'd',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' | 'ȅ' | 'ȇ' => 'e',
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => 'g',
        'ĥ' | 'ħ' => 'h',
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ǐ' | 'ȉ' | 'ȋ' => 'i',
        'ĵ' => 'j',
        'ķ' => 'k',
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => 'l',
        'ñ' | 'ń' | 'ņ' | 'ň' | 'ŋ' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' | 'ǒ' | 'ȍ' | 'ȏ' => 'o',
        'ŕ' | 'ŗ' | 'ř' => 'r',
        'ś' | 'ŝ' | 'ş' | 'š' => 's',
        'ţ' | 'ť' | 'ŧ' => 't',
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' | 'ǔ' | 'ȕ' | 'ȗ' => {
            'u'
        }
        'ŵ' => 'w',
        'ý' | 'ÿ' | 'ŷ' | 'ȳ' => 'y',
        'ź' | 'ż' | 'ž' => 'z',
        _ => return None,
    })
}

fn is_combining_mark(ch: char) -> bool {
    matches!(
        ch,
        '\u{0300}'..='\u{036F}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE20}'..='\u{FE2F}'
    )
}
