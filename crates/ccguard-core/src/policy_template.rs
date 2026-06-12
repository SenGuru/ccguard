//! Business-type policy templates — the on-ramp for a non-technical SMB owner.
//!
//! Each template is a FILLED business description in the owner's voice, ready to
//! edit a noun or two. The wording deliberately teaches the admin to write the
//! anti-false-positive clause themselves ("a brand-new repo / unfamiliar client
//! name is still our work"), because that clause is the load-bearing defense
//! against the classifier wrongly flagging new work as personal. Pure constants.

/// One business-type starter policy.
#[derive(Debug, Clone, Copy)]
pub struct PolicyTemplate {
    pub key: &'static str,
    pub name: &'static str,
    pub business_desc: &'static str,
    pub work_allowed: &'static str,
    pub personal_examples: &'static str,
}

/// All shipped templates.
pub fn all() -> &'static [PolicyTemplate] {
    TEMPLATES
}

/// Look one up by key.
pub fn by_key(key: &str) -> Option<&'static PolicyTemplate> {
    TEMPLATES.iter().find(|t| t.key == key)
}

const TEMPLATES: &[PolicyTemplate] = &[
    PolicyTemplate {
        key: "software_agency",
        name: "Software / dev agency",
        business_desc: "We're a software agency. Our work is building and maintaining client \
projects — web apps, APIs, mobile apps, and internal tools — across many languages and stacks. \
We onboard new clients constantly, so a brand-new repo or an unfamiliar client/project name is \
still our work, not a personal side-project.",
        work_allowed: "Any client project (current or prospective), our own internal tools and \
scripts, and learning/spikes done in service of client work. NOT a developer's personal \
side-projects, job applications, or hobby code.",
        personal_examples: "a personal portfolio site, a hobby game, leetcode practice, a resume",
    },
    PolicyTemplate {
        key: "saas_product",
        name: "SaaS product company",
        business_desc: "We build and run one main SaaS product plus its supporting services \
(API, web app, infra, internal admin tools, data pipelines). Refactors, new features, new \
microservices, prototypes, and bug-hunting on any of these are our work.",
        work_allowed: "Any work on our product, its services, infrastructure, internal tooling, \
or research/spikes for the product. NOT personal projects, freelance for other companies, or \
job hunting.",
        personal_examples: "a freelance gig for another company, a personal startup idea, a hobby app",
    },
    PolicyTemplate {
        key: "ecommerce_shopify",
        name: "E-commerce / Shopify store",
        business_desc: "We run an online store. Our development work is our storefront theme, \
custom Shopify apps/scripts, integrations (payments, shipping, inventory), and data/reporting \
for the shop. New experiments and a fresh repo for a new integration are still store work.",
        work_allowed: "Anything that builds, fixes, or analyses our store, its theme, apps, \
integrations, or reports. NOT personal coding projects or side businesses.",
        personal_examples: "a separate personal store, a hobby project, a personal blog",
    },
    PolicyTemplate {
        key: "marketing_agency",
        name: "Marketing / growth agency",
        business_desc: "We're a marketing agency; our devs build landing pages, campaign sites, \
tracking/analytics, automations, and small tools for clients. New client = new repo, which is \
still our work.",
        work_allowed: "Any client campaign, site, automation, or internal marketing tool, and \
spikes for them. NOT personal side-projects or job hunting.",
        personal_examples: "a personal newsletter site, a hobby project, freelance for someone else",
    },
    PolicyTemplate {
        key: "accounting_proservices",
        name: "Accounting / professional services",
        business_desc: "We're a professional-services firm; our developers build and maintain \
internal tools, client deliverables, spreadsheets-as-code, integrations with our practice \
software, and reporting/automation. Internal scripts and one-off client tools are our work even \
in new folders.",
        work_allowed: "Any internal tool, client deliverable, integration, or automation for the \
firm, plus learning in service of those. NOT personal projects or unrelated freelance.",
        personal_examples: "personal finance scripts, a hobby app, job-application code",
    },
    PolicyTemplate {
        key: "game_studio",
        name: "Game studio",
        business_desc: "We make games. Our work is our game projects, engine/tooling code, build \
pipelines, and prototypes — including brand-new prototype repos, which are still studio work.",
        work_allowed: "Any studio game, tool, pipeline, or prototype. NOT a developer's personal \
game made on the side, or unrelated hobby code.",
        personal_examples: "a solo game made outside the studio, a personal mod, job-hunt code",
    },
    PolicyTemplate {
        key: "internal_it",
        name: "Internal IT / MSP",
        business_desc: "We run internal IT (or manage IT for clients). Our work is automation \
scripts, infrastructure-as-code, integrations, monitoring, and internal/admin tools — frequently \
in new, throwaway-looking folders that are nonetheless company work.",
        work_allowed: "Any internal or client automation, infra, integration, or admin tool, and \
troubleshooting/spikes for them. NOT personal projects or job applications.",
        personal_examples: "a personal homelab project, a hobby script, a resume site",
    },
    PolicyTemplate {
        key: "other",
        name: "Other / general",
        business_desc: "Describe what your business does and what your developers build. Be \
concrete about the kinds of code that count as your work — and note that new or unfamiliar \
repos/projects are still your work, since the AI must never treat 'new' or 'unfamiliar' as a \
personal signal.",
        work_allowed: "List what Claude Code is allowed to be used for at your company, and what \
is out of scope (e.g. personal side-projects, job hunting).",
        personal_examples: "a personal side-project, a hobby app, job-application code",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_are_complete_and_keyed() {
        assert!(all().len() >= 6);
        for t in all() {
            assert!(!t.key.is_empty() && !t.name.is_empty());
            assert!(!t.business_desc.is_empty() && !t.work_allowed.is_empty());
        }
        // keys are unique
        let mut keys: Vec<_> = all().iter().map(|t| t.key).collect();
        keys.sort_unstable();
        let len = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), len, "template keys must be unique");
    }

    #[test]
    fn by_key_finds_and_misses() {
        assert_eq!(by_key("software_agency").unwrap().name, "Software / dev agency");
        assert!(by_key("nope").is_none());
    }

    #[test]
    fn templates_teach_the_anti_false_positive_clause() {
        // Every non-"other" template names that new/unfamiliar is still work.
        for t in all().iter().filter(|t| t.key != "other") {
            let low = t.business_desc.to_ascii_lowercase();
            assert!(
                low.contains("new") || low.contains("unfamiliar") || low.contains("fresh"),
                "template {} should teach the new-is-still-work clause",
                t.key
            );
        }
    }
}
