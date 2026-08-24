use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use icu_segmenter::WordSegmenter;
use icu_segmenter::options::WordBreakInvariantOptions;
use strum::IntoEnumIterator;
use unicode_normalization::UnicodeNormalization;

use crate::Cursor;
use crate::error::Result;
use crate::eval::{Evaluation, Evaluator, Scoreboard};
use crate::windows::{ResourceId, load_string, main_cpl};

static M: LazyLock<HashMap<Cursor, Vec<HashSet<String>>>> = LazyLock::new(|| {
    HashMap::from_iter(
        [
            (
                Cursor::NormalSelect,
                vec![vec!["normal", "select"], vec!["arrow"]],
            ),
            (Cursor::Busy, vec![vec!["busy"], vec!["wait"]]),
            (
                Cursor::WorkingInBackground,
                vec![vec!["working", "in", "background"], vec!["app", "starting"]],
            ),
            (Cursor::Unavailable, vec![vec!["unavailable"], vec!["no"]]),
            (
                Cursor::TextSelect,
                vec![vec!["text", "select"], vec!["i", "beam"]],
            ),
            (
                Cursor::PrecisionSelect,
                vec![vec!["precision", "select"], vec!["crosshair"]],
            ),
            (
                Cursor::VerticalResize,
                vec![vec!["vertical", "resize"], vec!["size", "ns"]],
            ),
            (
                Cursor::HorizontalResize,
                vec![vec!["horizontal", "resize"], vec!["size", "we"]],
            ),
            (
                Cursor::DiagonalResize1,
                vec![vec!["diagonal", "resize", "1"], vec!["size", "nwse"]],
            ),
            (
                Cursor::DiagonalResize2,
                vec![vec!["diagonal", "resize", "2"], vec!["size", "nesw"]],
            ),
            (Cursor::Move, vec![vec!["move"], vec!["size", "all"]]),
            (
                Cursor::HelpSelect,
                vec![vec!["help", "select"], vec!["help"]],
            ),
            (
                Cursor::Handwriting,
                vec![vec!["handwriting"], vec!["nw", "pen"]],
            ),
            (
                Cursor::AlternateSelect,
                vec![vec!["alternate", "select"], vec!["up", "arrow"]],
            ),
            (
                Cursor::LinkSelect,
                vec![vec!["link", "select"], vec!["hand"]],
            ),
            (
                Cursor::LocationSelect,
                vec![vec!["location", "select"], vec!["pin"]],
            ),
            (
                Cursor::PersonSelect,
                vec![vec!["person", "select"], vec!["person"]],
            ),
        ]
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                v.into_iter()
                    .map(|v| v.into_iter().map(|s| s.to_string()).collect())
                    .collect(),
            )
        }),
    )
});

pub struct WordEvaluator {
    dict: HashMap<Cursor, Vec<HashSet<String>>>,
}

impl WordEvaluator {
    fn tokenize(s: &str) -> HashSet<String> {
        s.to_lowercase()
            .nfkc()
            .collect::<String>()
            .trim()
            .trim_start_matches(|c: char| c.is_ascii_digit())
            .split(|c: char| c.is_whitespace() || c == '_')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    }

    fn pascal_case_tokenize(s: &str) -> HashSet<String> {
        split_pascal_case(
            s.nfkc()
                .collect::<String>()
                .trim()
                .trim_start_matches(|c: char| c.is_ascii_digit()),
        )
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
    }

    fn icu_tokenize(s: &str) -> HashSet<String> {
        let segmenter = WordSegmenter::new_auto(WordBreakInvariantOptions::default());
        let mut start = 0;
        let mut set: HashSet<String> = HashSet::new();
        let s = s.nfkc().collect::<String>();
        let s = s.trim().trim_start_matches(|c: char| c.is_ascii_digit());

        for end in segmenter.segment_str(s) {
            let word = s[start..end].trim();

            if !word.is_empty() {
                set.insert(word.to_lowercase());
            }

            start = end;
        }

        set
    }

    pub fn new() -> Result<Self> {
        let main_cpl = main_cpl()?;
        let mut m = M.clone();

        for cursor in Cursor::iter() {
            let cursor_resource_id = ResourceId::from(&cursor);
            let words = m.entry(cursor).or_default();

            let Some(display_name) = load_string(main_cpl, &cursor_resource_id) else {
                continue;
            };
            let display_name = display_name
                .trim_end_matches(|c| c == '\0')
                .replace('/', "");

            words.push(Self::tokenize(&display_name));
            words.push(Self::pascal_case_tokenize(&display_name));
            words.push(Self::icu_tokenize(&display_name));
        }

        Ok(Self { dict: m })
    }
}

fn is_upper_or_digit(c: char) -> bool {
    c.is_uppercase() || c.is_ascii_digit()
}

fn split_pascal_case(s: &str) -> Vec<&str> {
    let chars: Vec<_> = s.char_indices().collect();

    let mut boundaries = Vec::new();

    for i in 1..chars.len() {
        let (_, current) = chars[i];
        let (_, prev) = chars[i - 1];

        let next = chars.get(i + 1).map(|(_, c)| *c);

        if is_upper_or_digit(current)
            && (prev.is_lowercase() || next.is_some_and(|c| c.is_lowercase()))
        {
            boundaries.push(chars[i].0);
        }
    }

    let mut result = Vec::new();
    let mut start = 0;

    for boundary in boundaries {
        result.push(&s[start..boundary]);
        start = boundary;
    }

    result.push(&s[start..]);

    result
}

impl WordEvaluator {
    fn evaluate_words(&self, words: &HashSet<String>) -> Scoreboard {
        let mut scoreboard = Scoreboard::default();

        for (k, v) in &self.dict {
            let mut best = None;

            for expected in v {
                let matched = expected.intersection(&words).count();

                if matched == 0 {
                    continue;
                }

                let recall = matched as f64 / expected.len() as f64;
                let precision = matched as f64 / words.len() as f64;

                let evaluation = Evaluation {
                    total: recall + precision,
                    max: 2.0,
                };

                if best
                    .as_ref()
                    .is_none_or(|best: &Evaluation| evaluation.normalized() > best.normalized())
                {
                    best = Some(evaluation);
                }
            }

            if let Some(best) = best {
                scoreboard.add(k.clone(), &best);
            }
        }

        scoreboard
    }
}

impl Evaluator for WordEvaluator {
    fn evaluate(&self, path: &std::path::Path) -> Option<Scoreboard> {
        let Some(words) = path.file_stem().and_then(|n| n.to_str()) else {
            return None;
        };

        let mut scoreboard = Scoreboard::default();

        scoreboard.merge_best(self.evaluate_words(&Self::tokenize(words)));
        scoreboard.merge_best(self.evaluate_words(&Self::pascal_case_tokenize(words)));
        scoreboard.merge_best(self.evaluate_words(&Self::icu_tokenize(words)));

        Some(scoreboard)
    }
}
