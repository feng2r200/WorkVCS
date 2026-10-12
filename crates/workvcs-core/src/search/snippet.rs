use tiktoken_rs::o200k_base_singleton;

pub const SNIPPET_POLICY_VERSION: &str = "o200k-ordinary-128-24-all-query-lead-merge-v2";
const SNIPPET_WINDOW_TOKENS: usize = 128;
const SNIPPET_OVERLAP_TOKENS: usize = 24;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SnippetSpan {
    pub byte_start: usize,
    pub byte_end: usize,
}

pub(crate) fn snippet_spans(statement: &str) -> Vec<SnippetSpan> {
    if statement.is_empty() {
        return Vec::new();
    }
    let encoding = o200k_base_singleton();
    let tokens = encoding.encode_ordinary(statement);
    if tokens.is_empty() {
        return vec![SnippetSpan {
            byte_start: 0,
            byte_end: statement.len(),
        }];
    }

    let mut token_byte_boundaries = Vec::with_capacity(tokens.len() + 1);
    token_byte_boundaries.push(0usize);
    let mut byte_offset = 0usize;
    for token in &tokens {
        let token_bytes = encoding
            .decode_bytes(&[*token])
            .expect("ordinary token emitted by this encoding must decode with the same encoding");
        byte_offset += token_bytes.len();
        token_byte_boundaries.push(byte_offset);
    }
    debug_assert_eq!(byte_offset, statement.len());

    let step = SNIPPET_WINDOW_TOKENS - SNIPPET_OVERLAP_TOKENS;
    let mut spans = Vec::new();
    for start_token in (0..tokens.len()).step_by(step) {
        let end_token = (start_token + SNIPPET_WINDOW_TOKENS).min(tokens.len());
        let span = SnippetSpan {
            byte_start: previous_char_boundary(statement, token_byte_boundaries[start_token]),
            byte_end: next_char_boundary(statement, token_byte_boundaries[end_token]),
        };
        if spans.last() != Some(&span) {
            spans.push(span);
        }
        if end_token == tokens.len() {
            break;
        }
    }
    spans
}

fn previous_char_boundary(statement: &str, mut offset: usize) -> usize {
    while !statement.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn next_char_boundary(statement: &str, mut offset: usize) -> usize {
    while !statement.is_char_boundary(offset) {
        offset += 1;
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::snippet_spans;

    #[test]
    fn token_windows_cross_paragraphs_overlap_and_preserve_utf8_ranges() {
        let statement = format!("{}\n\n{}", "前文证据 ".repeat(40), "后文结论 ".repeat(180));
        let spans = snippet_spans(&statement);
        assert!(spans.len() >= 3);
        assert!(
            spans
                .iter()
                .any(|span| { statement[span.byte_start..span.byte_end].contains("\n\n") })
        );
        assert!(spans[0].byte_end > spans[1].byte_start);
        for span in spans {
            assert!(statement.is_char_boundary(span.byte_start));
            assert!(statement.is_char_boundary(span.byte_end));
            assert!(!statement[span.byte_start..span.byte_end].is_empty());
        }
    }

    #[test]
    fn short_statement_remains_one_exact_range() {
        let statement = "引用的错误陈述。随后明确否定：该陈述已经失效。";
        assert_eq!(
            snippet_spans(statement),
            vec![super::SnippetSpan {
                byte_start: 0,
                byte_end: statement.len(),
            }]
        );
    }
}
