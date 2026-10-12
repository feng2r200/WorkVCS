use jieba_rs::Jieba;
use std::collections::HashSet;
use std::sync::OnceLock;

pub const TOKENIZER_VERSION: &str =
    "jieba-rs-0.10.4-hmm-false-cjk-bigram-ascii-fts5-unicode61-default-v1";

const STOPWORDS: &[&str] = &[
    "的", "了", "是", "在", "和", "与", "对", "把", "目前", "当前", "进行", "这个", "需要", "我们",
    "是否", "问题", "情况", "应该", "内容", "可以", "什么", "哪些", "如何", "一个", "一些", "the",
    "a", "an", "to", "of", "in", "and", "for", "is", "are", "this", "that",
];

fn jieba() -> &'static Jieba {
    static JIEBA: OnceLock<Jieba> = OnceLock::new();
    JIEBA.get_or_init(Jieba::new)
}

pub fn lexical_tokens(text: &str) -> Vec<String> {
    let stopwords = STOPWORDS.iter().copied().collect::<HashSet<_>>();
    let mut output = ascii_tokens(text);

    for run in cjk_runs(text) {
        output.extend(
            jieba()
                .cut(run, false)
                .into_iter()
                .filter(|token| !token.word.trim().is_empty())
                .map(|token| token.word.to_owned()),
        );
        let chars = run.chars().collect::<Vec<_>>();
        output.extend(chars.windows(2).map(|pair| pair.iter().collect::<String>()));
    }

    output
        .into_iter()
        .filter(|token| token.chars().count() > 1 && !stopwords.contains(token.as_str()))
        .collect()
}

pub fn fts_query(text: &str) -> Option<String> {
    let mut seen = HashSet::new();
    let terms = lexical_tokens(text)
        .into_iter()
        .filter(|term| seen.insert(term.clone()))
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>();
    (!terms.is_empty()).then(|| terms.join(" OR "))
}

fn ascii_tokens(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut output = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_alphanumeric() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_alphanumeric() {
            index += 1;
        }
        loop {
            if index + 1 >= bytes.len()
                || !matches!(bytes[index], b'_' | b'-')
                || !bytes[index + 1].is_ascii_alphanumeric()
            {
                break;
            }
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_alphanumeric() {
                index += 1;
            }
        }
        output.push(text[start..index].to_ascii_lowercase());
    }
    output
}

fn cjk_runs(text: &str) -> Vec<&str> {
    let mut output = Vec::new();
    let mut start = None;
    for (offset, character) in text.char_indices() {
        let is_cjk = ('\u{4e00}'..='\u{9fff}').contains(&character);
        match (start, is_cjk) {
            (None, true) => start = Some(offset),
            (Some(run_start), false) => {
                output.push(&text[run_start..offset]);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(run_start) = start {
        output.push(&text[run_start..]);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{fts_query, lexical_tokens};

    #[test]
    fn preserves_ascii_technical_identifiers_and_cjk_bigrams() {
        let tokens = lexical_tokens("ProjectRef-v2 的 state_digest 与工作区检索");
        assert!(tokens.contains(&"projectref-v2".to_owned()));
        assert!(tokens.contains(&"state_digest".to_owned()));
        assert!(tokens.contains(&"工作".to_owned()));
        assert!(tokens.contains(&"检索".to_owned()));
        assert!(!tokens.contains(&"的".to_owned()));
    }

    #[test]
    fn query_terms_are_stably_deduplicated() {
        let query = fts_query("WorkVCS WorkVCS 工作工作").expect("terms");
        assert_eq!(query.matches("\"workvcs\"").count(), 1);
        assert_eq!(query.matches("\"工作\"").count(), 1);
    }

    #[test]
    fn matches_frozen_python_jieba_policy_terms_and_counts() {
        let cases = [
            (
                "ProjectRef-v2 的 state_digest 与工作区检索",
                vec![
                    "projectref-v2",
                    "state_digest",
                    "工作",
                    "检索",
                    "与工",
                    "工作",
                    "作区",
                    "区检",
                    "检索",
                ],
            ),
            (
                "WorkVCS的ReplayCache避免每个hit重复record_at回放",
                vec![
                    "workvcs",
                    "replaycache",
                    "hit",
                    "record_at",
                    "避免",
                    "每个",
                    "避免",
                    "免每",
                    "每个",
                    "重复",
                    "重复",
                    "回放",
                    "回放",
                ],
            ),
            (
                "中文jieba分词HMM=false，加双字gram和ASCII技术标识符",
                vec![
                    "jieba",
                    "hmm",
                    "false",
                    "gram",
                    "ascii",
                    "中文",
                    "中文",
                    "分词",
                    "分词",
                    "双字",
                    "加双",
                    "双字",
                    "技术",
                    "标识符",
                    "技术",
                    "术标",
                    "标识",
                    "识符",
                ],
            ),
            (
                "ADR-0525规定FTS5 BM25回读canonical statement",
                vec![
                    "adr-0525",
                    "fts5",
                    "bm25",
                    "canonical",
                    "statement",
                    "规定",
                    "规定",
                    "回读",
                ],
            ),
        ];
        for (text, expected) in cases {
            assert_eq!(lexical_tokens(text), expected, "token mismatch for {text}");
        }
    }
}
