//! AI-assisted policy drafting — the meta-prompt that turns a one-sentence business
//! description into the three policy fields, which the admin then edits. Pure: builds
//! the prompt string; the actual call goes through the same local-Claude-Code /
//! server-API path as classification. The hard guard against hallucinating company
//! names/domains is load-bearing — an invented `acme.com` would poison every future
//! verdict.

/// Build the drafting prompt from the owner's one-liner.
pub fn draft_prompt(one_liner: &str) -> String {
    format!(
        "You help a non-technical business owner write a policy that an AI will use to \
classify each of their developers' coding sessions as company WORK or PERSONAL.\n\n\
The owner described their business in one line:\n\"{}\"\n\n\
Expand this into three short, plain-English fields:\n\
- business_desc: what the business does and what its real work looks like in code. \
INCLUDE a sentence that new or unfamiliar repos/projects are still the company's work \
(the AI must never treat 'new' or 'unfamiliar' as a personal signal).\n\
- work_allowed: what Claude Code is allowed to be used for, and what is out of scope \
(e.g. personal side-projects, job hunting).\n\
- personal_examples: 2-4 short examples of what is NOT this business's work.\n\n\
HARD RULES:\n\
- Do NOT invent company names, domains, product names, or ticket prefixes the owner \
did not give you. Use only what is in their description; stay generic otherwise.\n\
- Keep each field a few sentences, in the owner's voice.\n\
Return ONLY a JSON object with keys business_desc, work_allowed, personal_examples.",
        one_liner.trim().replace('"', "'")
    )
}

/// Parse the drafted fields out of the model reply (tolerant of surrounding prose).
pub fn parse_draft(raw: &str) -> Option<(String, String, String)> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    let v: serde_json::Value = serde_json::from_str(raw.get(start..=end)?).ok()?;
    let get = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let bd = get("business_desc");
    let wa = get("work_allowed");
    let pe = get("personal_examples");
    if bd.is_empty() && wa.is_empty() {
        None
    } else {
        Some((bd, wa, pe))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_includes_one_liner_and_no_hallucination_guard() {
        let p = draft_prompt("I run a Shopify store with two devs");
        assert!(p.contains("Shopify store with two devs"));
        assert!(p.contains("Do NOT invent company names"));
        assert!(p.contains("new") && p.contains("unfamiliar"));
    }

    #[test]
    fn prompt_neutralizes_quotes_in_the_one_liner() {
        let p = draft_prompt("we build \"widgets\" for clients");
        // the embedded quotes are turned to single quotes so they don't break the frame
        assert!(p.contains("'widgets'"));
    }

    #[test]
    fn parse_extracts_three_fields() {
        let raw = "Here you go:\n{\"business_desc\":\"We build apps.\",\"work_allowed\":\"Client work.\",\"personal_examples\":\"hobby app\"}";
        let (bd, wa, pe) = parse_draft(raw).unwrap();
        assert_eq!(bd, "We build apps.");
        assert_eq!(wa, "Client work.");
        assert_eq!(pe, "hobby app");
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(parse_draft("no json here").is_none());
        assert!(parse_draft("{\"business_desc\":\"\",\"work_allowed\":\"\"}").is_none());
    }
}
