use super::model::read_regular_file;
use crate::error::{Result, WorkVcsError};
use std::path::Path;
use tokenizers::Tokenizer;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ModelWindow {
    pub byte_start: usize,
    pub byte_end: usize,
    pub token_start: usize,
    pub token_end: usize,
    pub model_tokens: usize,
    pub text: String,
    pub prefixed_text: String,
}

pub(super) fn load_tokenizer(model_path: &Path) -> Result<Tokenizer> {
    let bytes = read_regular_file(model_path.join("tokenizer.json"))?;
    let mut tokenizer = Tokenizer::from_bytes(&bytes).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot load verified tokenizer {}: {error}",
            model_path.join("tokenizer.json").display()
        ))
    })?;
    tokenizer.with_truncation(None).map_err(tokenizer_error)?;
    tokenizer.with_padding(None);
    Ok(tokenizer)
}

pub(super) fn model_windows(
    tokenizer: &Tokenizer,
    text: &str,
    prefix: &str,
    max_tokens: usize,
    overlap_tokens: usize,
) -> Result<Vec<ModelWindow>> {
    if max_tokens == 0 {
        return Err(WorkVcsError::QueryInvalid(
            "model context length must be greater than zero".to_owned(),
        ));
    }
    let raw = tokenizer.encode(text, false).map_err(tokenizer_error)?;
    let offsets = raw.get_offsets();
    if offsets.is_empty() {
        let prefixed_text = format!("{prefix}{text}");
        let model_tokens = encoded_len(tokenizer, &prefixed_text)?;
        if model_tokens > max_tokens {
            return Err(WorkVcsError::QueryInvalid(format!(
                "model tokenizer produced {model_tokens} tokens for text without addressable offsets, exceeding context {max_tokens}"
            )));
        }
        return Ok(vec![ModelWindow {
            byte_start: 0,
            byte_end: text.len(),
            token_start: 0,
            token_end: 0,
            model_tokens,
            text: text.to_owned(),
            prefixed_text,
        }]);
    }

    let mut windows = Vec::new();
    let mut token_start = 0_usize;
    while token_start < offsets.len() {
        let mut token_end = offsets.len().min(token_start.saturating_add(max_tokens));
        let byte_start = if token_start == 0 {
            0
        } else {
            offsets[token_start].0
        };
        let mut chosen = None;
        while token_end > token_start {
            let byte_end = if token_end == offsets.len() {
                text.len()
            } else {
                offsets[token_end - 1].1
            };
            if byte_start > byte_end
                || byte_end > text.len()
                || !text.is_char_boundary(byte_start)
                || !text.is_char_boundary(byte_end)
            {
                return Err(WorkVcsError::StorageFailure(
                    "tokenizer returned a non-canonical UTF-8 byte offset".to_owned(),
                ));
            }
            let window_text = &text[byte_start..byte_end];
            let prefixed_text = format!("{prefix}{window_text}");
            let model_tokens = encoded_len(tokenizer, &prefixed_text)?;
            if model_tokens <= max_tokens {
                chosen = Some((
                    byte_end,
                    window_text.to_owned(),
                    prefixed_text,
                    model_tokens,
                ));
                break;
            }
            token_end -= 1;
        }
        let Some((byte_end, window_text, prefixed_text, model_tokens)) = chosen else {
            return Err(WorkVcsError::QueryInvalid(format!(
                "one tokenizer unit plus the required prefix cannot fit model context {max_tokens}"
            )));
        };
        windows.push(ModelWindow {
            byte_start,
            byte_end,
            token_start,
            token_end,
            model_tokens,
            text: window_text,
            prefixed_text,
        });
        if token_end == offsets.len() {
            break;
        }
        let next = token_end.saturating_sub(overlap_tokens);
        token_start = if next <= token_start {
            token_start + 1
        } else {
            next
        };
    }
    validate_complete_coverage(text, &windows)?;
    Ok(windows)
}

pub(super) fn pair_token_count(tokenizer: &Tokenizer, query: &str, passage: &str) -> Result<usize> {
    tokenizer
        .encode((query, passage), true)
        .map(|encoding| encoding.len())
        .map_err(tokenizer_error)
}

fn encoded_len(tokenizer: &Tokenizer, text: &str) -> Result<usize> {
    tokenizer
        .encode(text, true)
        .map(|encoding| encoding.len())
        .map_err(tokenizer_error)
}

fn tokenizer_error(error: impl std::fmt::Display) -> WorkVcsError {
    WorkVcsError::StorageFailure(format!("semantic tokenizer failed: {error}"))
}

fn validate_complete_coverage(text: &str, windows: &[ModelWindow]) -> Result<()> {
    if windows.is_empty()
        || windows[0].byte_start != 0
        || windows.last().map(|window| window.byte_end) != Some(text.len())
    {
        return Err(WorkVcsError::StorageFailure(
            "semantic tokenizer windows do not cover the complete source text".to_owned(),
        ));
    }
    let mut covered_end = 0_usize;
    for window in windows {
        if window.byte_start > covered_end
            || window.byte_start > window.byte_end
            || window.byte_end > text.len()
            || !text.is_char_boundary(window.byte_start)
            || !text.is_char_boundary(window.byte_end)
            || text[window.byte_start..window.byte_end] != window.text
        {
            return Err(WorkVcsError::StorageFailure(
                "semantic tokenizer window coverage or UTF-8 identity is invalid".to_owned(),
            ));
        }
        covered_end = covered_end.max(window.byte_end);
    }
    if covered_end != text.len() {
        return Err(WorkVcsError::StorageFailure(
            "semantic tokenizer windows omit source bytes".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenizers::models::wordlevel::WordLevel;
    use tokenizers::pre_tokenizers::whitespace::Whitespace;

    fn tokenizer() -> Tokenizer {
        let vocabulary = [
            ("[UNK]".to_owned(), 0),
            ("query".to_owned(), 1),
            (":".to_owned(), 2),
            ("one".to_owned(), 3),
            ("two".to_owned(), 4),
            ("three".to_owned(), 5),
            ("four".to_owned(), 6),
            ("five".to_owned(), 7),
        ]
        .into_iter()
        .collect();
        let model = WordLevel::builder()
            .vocab(vocabulary)
            .unk_token("[UNK]".to_owned())
            .build()
            .expect("word level model");
        let mut tokenizer = Tokenizer::new(model);
        tokenizer.with_pre_tokenizer(Some(Whitespace));
        tokenizer
    }

    #[test]
    fn windows_cover_every_byte_without_truncation() {
        let text = "one two three four five";
        let windows = model_windows(&tokenizer(), text, "query: ", 5, 1).expect("windows");
        assert_eq!(windows.first().unwrap().byte_start, 0);
        assert_eq!(windows.last().unwrap().byte_end, text.len());
        assert!(windows.len() > 1);
        assert!(windows.iter().all(|window| window.model_tokens <= 5));
    }
}
