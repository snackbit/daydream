//! Model-independent byte bounds leave room for provider prefixes and specials.
const MAX_BYTES: usize = 480;
const OVERLAP_BYTES: usize = 96;
pub const FRAGMENT_VERSION: &str = "canonical-fragments480bytes-overlap96-fullpassage-v2";

pub fn text_fragments(text: &str) -> Vec<&str> {
    if text.len() <= MAX_BYTES {
        return vec![text];
    }
    let mut output = vec![];
    let mut start = 0;
    while start < text.len() {
        let mut ceiling = (start + MAX_BYTES).min(text.len());
        while !text.is_char_boundary(ceiling) {
            ceiling -= 1;
        }
        let mut floor = ceiling.saturating_sub(96).max(start + 1);
        while !text.is_char_boundary(floor) {
            floor += 1;
        }
        let boundary = |sentence: bool| {
            text[floor..ceiling]
                .char_indices()
                .rev()
                .find_map(|(offset, character)| {
                    let index = floor + offset;
                    let after = index + character.len_utf8();
                    if sentence {
                        (character == '\n'
                            || (matches!(character, '.' | '!' | '?')
                                && text[after..]
                                    .chars()
                                    .next()
                                    .is_some_and(char::is_whitespace)))
                        .then_some(after)
                    } else {
                        character.is_whitespace().then_some(index)
                    }
                })
        };
        let end = if ceiling == text.len() {
            ceiling
        } else {
            boundary(true)
                .or_else(|| boundary(false))
                .unwrap_or(ceiling)
        };
        let fragment = text[start..end].trim();
        if !fragment.is_empty() {
            output.push(fragment);
        }
        if end == text.len() {
            break;
        }
        let mut earliest = end.saturating_sub(OVERLAP_BYTES).max(start + 1);
        while !text.is_char_boundary(earliest) {
            earliest += 1;
        }
        // Start at a complete word inside the bounded overlap. Long words and
        // scripts without whitespace use a valid Unicode boundary instead.
        start = if text[..earliest]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
        {
            earliest
        } else {
            text[earliest..end]
                .char_indices()
                .find(|(_, character)| character.is_whitespace())
                .map(|(offset, character)| earliest + offset + character.len_utf8())
                .unwrap_or(earliest)
        };
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragments_preserve_rare_passages_and_unicode_boundaries() {
        let text = format!(
            "{} A rare synthetic clause describes Rückkopplung in a photovoltaic inverter. {}",
            "Synthetic commonplace sentence. ".repeat(80),
            "Weitere gewöhnliche deutsche Wörter. ".repeat(80)
        );
        let fragments = text_fragments(&text);
        assert!(fragments.len() > 3);
        assert!(fragments.iter().all(|part| part.len() <= MAX_BYTES));
        assert!(fragments
            .iter()
            .any(|part| part.contains("Rückkopplung in a photovoltaic inverter")));
        for text in ["界".repeat(3001), "😀".repeat(2001)] {
            let fragments = text_fragments(&text);
            assert!(fragments.iter().all(|part| part.len() <= MAX_BYTES));
        }
    }
}
