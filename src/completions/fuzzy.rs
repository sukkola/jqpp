use crate::completions::CompletionItem;

pub fn fuzzy_score(token: &str, label: &str) -> Option<i32> {
    if token.is_empty() {
        return None;
    }

    let token_chars: Vec<char> = token.chars().map(|c| c.to_ascii_lowercase()).collect();
    let label_chars: Vec<char> = label.chars().map(|c| c.to_ascii_lowercase()).collect();
    let token_lower: String = token_chars.iter().collect();
    let label_lower: String = label_chars.iter().collect();

    let mut matched_positions = Vec::with_capacity(token_chars.len());
    let mut search_start = 0usize;

    for tc in token_chars {
        let rel = label_chars[search_start..]
            .iter()
            .position(|&lc| lc == tc)?;
        let abs = search_start + rel;
        matched_positions.push(abs);
        search_start = abs + 1;
    }

    let contiguous_bonus = matched_positions
        .windows(2)
        .filter(|w| w[1] == w[0] + 1)
        .count() as i32
        * 2;

    let gap_penalty = matched_positions
        .windows(2)
        .map(|w| (w[1] as i32 - w[0] as i32 - 1).max(0))
        .sum::<i32>();

    let first = matched_positions[0] as i32;
    let early_bonus = (20 - first).max(0);
    let substring_boost = if label_lower.contains(&token_lower) {
        8
    } else {
        0
    };

    Some(
        token.chars().count() as i32 + contiguous_bonus + early_bonus + substring_boost
            - (gap_penalty * 2),
    )
}

pub fn fuzzy_completions(token: &str, items: &[CompletionItem]) -> Vec<CompletionItem> {
    if token.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(i32, CompletionItem)> = items
        .iter()
        .filter_map(|item| fuzzy_score(token, &item.label).map(|score| (score, item.clone())))
        .collect();

    scored.sort_by(|(score_a, item_a), (score_b, item_b)| {
        score_b
            .cmp(score_a)
            .then_with(|| item_a.label.cmp(&item_b.label))
    });

    scored
        .into_iter()
        .map(|(_, mut item)| {
            item.detail = Some(match item.detail {
                Some(d) => format!("~{}", d),
                None => "~".to_string(),
            });
            item
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            detail: Some("detail".to_string()),
            insert_text: label.to_string(),
        }
    }

    #[test]
    fn subsequence_matches_case_insensitive() {
        assert!(fuzzy_score("upcase", "ascii_upcase").is_some());
        assert!(fuzzy_score("UPCASE", "ascii_upcase").is_some());
    }

    #[test]
    fn non_subsequence_returns_none() {
        assert_eq!(fuzzy_score("xyz", "ascii_upcase"), None);
    }

    #[test]
    fn contiguous_scores_higher_than_spread() {
        let a = fuzzy_score("abc", "zzabczz").unwrap();
        let b = fuzzy_score("abc", "azbzcz").unwrap();
        assert!(a > b);
    }

    #[test]
    fn earlier_match_scores_higher_than_later_match() {
        let early = fuzzy_score("str", "str_suffix").unwrap();
        let late = fuzzy_score("str", "xxstr_suffix").unwrap();
        assert!(early > late);
    }

    #[test]
    fn empty_token_produces_no_fuzzy_results() {
        let out = fuzzy_completions("", &[item("tostring")]);
        assert!(out.is_empty());
    }

    #[test]
    fn closer_matches_rank_above_spread_matches() {
        let close = fuzzy_score("ame", "name").unwrap();
        let spread = fuzzy_score("ame", "a_b_m_c_e").unwrap();
        assert!(close > spread);
    }
}
