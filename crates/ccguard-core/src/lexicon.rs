//! Learned work-vocabulary evidence (triage channel 2).
//!
//! From the sessions a HUMAN has confirmed as work, learn the vocabulary that is
//! distinctive of this company's work (vs the background of all other sessions),
//! and measure how lexically similar a new session is to those confirmed-work
//! sessions. Both outputs are EVIDENCE handed to the AI judge — never a
//! classifier: the judge weighs them alongside everything else and stays the
//! sole decision-maker (AI-primary by design).
//!
//! Grounding rule: only human-confirmed labels feed this module. Using AI labels
//! would let the judge amplify its own guesses (circularity) and would churn the
//! triage `input_digest` mid-sweep as verdicts post (human labels only change on
//! an explicit dashboard action).
//!
//! Pure string math (term-frequency log-odds + cosine) — no model, no I/O, no
//! dependencies — so it runs identically on the server for every tenant.

use std::collections::{HashMap, HashSet};

/// Tokens shorter than this carry no signal ("a", "to", "is"...).
const MIN_TOKEN_LEN: usize = 3;
/// Tokens longer than this are hashes/paths/base64 noise.
const MAX_TOKEN_LEN: usize = 24;

/// Words too generic to ever be "distinctive of work" — common English plus the
/// vocabulary every Claude Code session contains regardless of subject.
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "can", "had", "her", "was", "one",
    "our", "out", "day", "get", "has", "him", "his", "how", "man", "new", "now", "old", "see",
    "two", "way", "who", "did", "its", "let", "she", "too", "use", "that", "with", "have",
    "this", "will", "your", "from", "they", "know", "want", "been", "good", "much", "some",
    "time", "very", "when", "come", "here", "just", "like", "long", "make", "many", "more",
    "only", "over", "such", "take", "than", "them", "well", "were", "what", "would", "about",
    "could", "there", "their", "which", "should", "into", "also", "because", "does", "doing",
    // dev-session background noise
    "file", "files", "code", "build", "create", "need", "needs", "using", "used", "run",
    "running", "please", "help", "claude", "session", "project", "projects", "work", "working",
    "thing", "things", "stuff", "yeah", "okay", "right", "actually", "something", "everything",
    "really", "still", "then", "than", "after", "before", "where", "while", "first", "last",
    // conversational filler — a developer's chat style is not work vocabulary
    "lets", "feel", "feels", "most", "sure", "kind", "kinda", "mean", "means", "wait",
    "gonna", "wanna", "want", "guess", "basically", "literally", "pretty", "maybe", "think",
];

/// Lowercase word tokens, length-bounded, stopword-filtered. Splits on anything
/// that isn't alphanumeric (so paths/URLs decompose into their words).
pub fn tokenize(text: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOPWORDS.iter().copied().collect();
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| {
            let n = t.chars().count();
            (MIN_TOKEN_LEN..=MAX_TOKEN_LEN).contains(&n)
                && !stop.contains(t)
                && !t.chars().all(|c| c.is_ascii_digit())
        })
        .map(str::to_string)
        .collect()
}

/// Document frequency: in how many docs does each term appear (at least once)?
fn doc_freq(docs: &[String]) -> HashMap<String, usize> {
    let mut df: HashMap<String, usize> = HashMap::new();
    for d in docs {
        let unique: HashSet<String> = tokenize(d).into_iter().collect();
        for t in unique {
            *df.entry(t).or_insert(0) += 1;
        }
    }
    df
}

/// The terms most distinctive of the confirmed-work docs relative to the
/// background docs, by smoothed log-odds of document frequency. A term must
/// appear in at least two work docs (or all of them, when fewer than two exist)
/// so a single odd session can't mint "work vocabulary" on its own.
pub fn distinctive_terms(
    work_docs: &[String],
    background_docs: &[String],
    max_terms: usize,
) -> Vec<String> {
    if work_docs.is_empty() || max_terms == 0 {
        return Vec::new();
    }
    let wf = doc_freq(work_docs);
    let bf = doc_freq(background_docs);
    let nw = work_docs.len() as f64;
    let nb = background_docs.len().max(1) as f64;
    let min_work_docs = 2.min(work_docs.len());

    let mut scored: Vec<(f64, String)> = wf
        .into_iter()
        .filter(|(_, c)| *c >= min_work_docs)
        .map(|(t, c)| {
            let p_work = (c as f64 + 0.5) / (nw + 1.0);
            let p_bg = (*bf.get(&t).unwrap_or(&0) as f64 + 0.5) / (nb + 1.0);
            ((p_work / p_bg).ln(), t)
        })
        .filter(|(s, _)| *s > 0.0)
        .collect();
    // Highest log-odds first; tie-break alphabetically so output is deterministic.
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1)));
    scored.into_iter().take(max_terms).map(|(_, t)| t).collect()
}

/// Which lexicon terms occur in this document (order of the lexicon = relevance order).
pub fn term_hits(doc: &str, lexicon: &[String]) -> Vec<String> {
    let toks: HashSet<String> = tokenize(doc).into_iter().collect();
    lexicon.iter().filter(|t| toks.contains(*t)).cloned().collect()
}

/// Term-frequency vector for cosine similarity.
fn tf(doc: &str) -> HashMap<String, f64> {
    let mut m: HashMap<String, f64> = HashMap::new();
    for t in tokenize(doc) {
        *m.entry(t).or_insert(0.0) += 1.0;
    }
    m
}

fn cosine(a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
    let dot: f64 = a
        .iter()
        .filter_map(|(t, va)| b.get(t).map(|vb| va * vb))
        .sum();
    if dot == 0.0 {
        return 0.0;
    }
    let na: f64 = a.values().map(|v| v * v).sum::<f64>().sqrt();
    let nb: f64 = b.values().map(|v| v * v).sum::<f64>().sqrt();
    dot / (na * nb)
}

/// Max cosine similarity of `doc` to any confirmed-work doc, with the index of
/// that nearest doc (so the caller can name it). `None` when there is nothing to
/// compare against or the doc is empty of tokens.
pub fn nearest_work(doc: &str, work_docs: &[String]) -> Option<(f32, usize)> {
    let dv = tf(doc);
    if dv.is_empty() || work_docs.is_empty() {
        return None;
    }
    work_docs
        .iter()
        .enumerate()
        .map(|(i, w)| (cosine(&dv, &tf(w)) as f32, i))
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn docs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn tokenize_filters_stopwords_and_short_tokens() {
        let t = tokenize("The quick conformal calibration of Florence-2 is a 99 thing");
        assert!(t.contains(&"conformal".to_string()));
        assert!(t.contains(&"calibration".to_string()));
        assert!(t.contains(&"florence".to_string()));
        assert!(!t.contains(&"the".to_string()));
        assert!(!t.contains(&"is".to_string()));
        assert!(!t.contains(&"99".to_string())); // pure digits dropped
        assert!(!t.contains(&"thing".to_string())); // dev-noise stopword
    }

    #[test]
    fn distinctive_terms_surface_work_vocabulary() {
        let work = docs(&[
            "grove signal mesh conformal calibration",
            "grove florence vision cascade conformal",
            "signal mesh activity taxonomy grove",
        ]);
        let bg = docs(&[
            "portfolio ideation mrr business",
            "fix samsung tv connectivity",
            "shopify functions migration revenue",
        ]);
        let lex = distinctive_terms(&work, &bg, 10);
        assert!(lex.contains(&"grove".to_string()));
        assert!(lex.contains(&"conformal".to_string()));
        // background-only vocabulary must not appear
        assert!(!lex.contains(&"shopify".to_string()));
    }

    #[test]
    fn single_doc_terms_need_all_docs_when_few() {
        // With >=2 work docs, a term in only ONE of them is excluded.
        let work = docs(&["solo oddball term", "grove grove grove"]);
        let lex = distinctive_terms(&work, &[], 20);
        assert!(!lex.contains(&"oddball".to_string()));
    }

    #[test]
    fn empty_work_docs_yield_empty_lexicon() {
        assert!(distinctive_terms(&[], &docs(&["x y z"]), 5).is_empty());
    }

    #[test]
    fn term_hits_respects_lexicon_order() {
        let lex = docs(&["taxonomy", "vision", "conformal"]);
        let hits = term_hits("designing the activity taxonomy with a vision model", &lex);
        assert_eq!(hits, vec!["taxonomy".to_string(), "vision".to_string()]);
    }

    #[test]
    fn nearest_work_finds_the_similar_doc() {
        let work = docs(&[
            "portfolio ideation mrr",
            "grove signal mesh activity taxonomy classifier",
        ]);
        let (sim, idx) = nearest_work("universal activity taxonomy classifier design", &work).unwrap();
        assert_eq!(idx, 1);
        assert!(sim > 0.3);
    }

    #[test]
    fn nearest_work_none_for_empty_inputs() {
        assert!(nearest_work("", &docs(&["a b c"])).is_none());
        assert!(nearest_work("hello conformal", &[]).is_none());
    }

    #[test]
    fn deterministic_output_order() {
        let work = docs(&["alpha beta gamma", "alpha beta gamma"]);
        let a = distinctive_terms(&work, &[], 10);
        let b = distinctive_terms(&work, &[], 10);
        assert_eq!(a, b);
    }
}
