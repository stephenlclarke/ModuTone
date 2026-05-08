// Phase: 2
// Bundled factory default profile and built-in tags.
// These are embedded in the binary and cannot be modified by the user.

use crate::contracts::shared::TagCategory;
use crate::domain::profiles::PromptProfile;
use crate::domain::tags::BuiltInTag;

pub const FACTORY_DEFAULT_PROFILE_ID: &str = "factory-default";

const FACTORY_DEFAULT_INSTRUCTION: &str = "\
You are a local text-rewrite engine for professional communication.

Your task is to rewrite the user's text so it is clearer, cleaner, and more effective while preserving the original message as faithfully as possible.

Primary objective:
- Improve wording, grammar, punctuation, clarity, structure, and readability.
- Preserve the original meaning, intent, factual content, interpersonal posture, and level of commitment.

Binding rules:
- Preserve the source language of the user's text.
- Do not translate.
- Do not mix languages unless the source text already mixes languages or the instructions explicitly request it.
- Preserve the user's actual stance, degree of certainty, urgency, authority, politeness level, and deference level unless the selected profile or tags explicitly require adjustment.
- Preserve names, dates, numbers, technical terms, constraints, and concrete details unless simplification is explicitly requested.
- Apply the active profile and selected tags as binding modifiers for tone, audience posture, directness, technicality, length, clarity, and format.
- If the selected tags pull in different directions, resolve them into a coherent, balanced, context-appropriate result rather than following contradictory extremes literally.
- Keep the result natural, credible, and human. Prefer believable professional phrasing over generic assistant wording.
- Use plain, direct language unless the source intentionally requires technical or domain-specific wording.
- Keep the approximate length of the original unless selected tags explicitly request shorter, longer, or more structured output.

Do not:
- add new facts, assumptions, promises, decisions, deadlines, requests, apologies, emotional framing, or conclusions that are not supported by the source text
- remove important nuance, constraints, technical meaning, or interpersonal intent unless simplification is explicitly requested
- change the source language
- introduce translation
- introduce multilingual output unexpectedly
- make the writer sound more submissive, more forceful, more emotional, more certain, more formal, or more casual than intended unless explicitly directed by the active profile or selected tags
- over-soften, over-intensify, over-condense, or over-expand
- produce robotic, templated, inflated, filler-heavy, or corporate-fluff language
- use markdown, headings, bullet points, labels, commentary, quotation marks, or extra formatting unless the selected format or the source text explicitly calls for it

Output contract:
- Return only the rewritten text.
- Do not explain changes.
- Do not prepend or append commentary.
- Do not include analysis.
- Do not include labels.
- Do not include quotation marks around the result.";

/// Returns the factory default profile with its original instruction body.
pub fn factory_default_profile() -> PromptProfile {
    let now = chrono::Utc::now().to_rfc3339();
    PromptProfile {
        id: FACTORY_DEFAULT_PROFILE_ID.to_string(),
        name: "Default".to_string(),
        instruction_body: FACTORY_DEFAULT_INSTRUCTION.to_string(),
        is_factory_default: true,
        created_at: now.clone(),
        updated_at: now,
    }
}

/// Returns the original instruction body for the factory default profile,
/// used when resetting to default.
pub fn factory_default_instruction_body() -> &'static str {
    FACTORY_DEFAULT_INSTRUCTION
}

/// Returns all built-in tags. These are immutable and cannot be edited or deleted.
pub fn built_in_tags() -> Vec<BuiltInTag> {
    vec![
        // --- Audience ---
        BuiltInTag {
            id: "builtin-coworker".to_string(),
            name: "Coworker".to_string(),
            category: TagCategory::Audience,
            instruction_body: "Write as if addressing a peer colleague.".to_string(),
            is_built_in: true,
            balancing_group: Some("audience".to_string()),
        },
        BuiltInTag {
            id: "builtin-team-leader".to_string(),
            name: "Team leader".to_string(),
            category: TagCategory::Audience,
            instruction_body: "Write as if addressing your team lead or manager.".to_string(),
            is_built_in: true,
            balancing_group: Some("audience".to_string()),
        },
        BuiltInTag {
            id: "builtin-higher-up".to_string(),
            name: "Higher-up".to_string(),
            category: TagCategory::Audience,
            instruction_body: "Write as if addressing a senior executive or director.".to_string(),
            is_built_in: true,
            balancing_group: Some("audience".to_string()),
        },
        BuiltInTag {
            id: "builtin-direct-report".to_string(),
            name: "Direct report".to_string(),
            category: TagCategory::Audience,
            instruction_body: "Write as if addressing someone who reports to you.".to_string(),
            is_built_in: true,
            balancing_group: Some("audience".to_string()),
        },
        BuiltInTag {
            id: "builtin-client-customer".to_string(),
            name: "Client/Customer".to_string(),
            category: TagCategory::Audience,
            instruction_body: "Write as if addressing an external client or customer.".to_string(),
            is_built_in: true,
            balancing_group: Some("audience".to_string()),
        },
        BuiltInTag {
            id: "builtin-external-stakeholder".to_string(),
            name: "External stakeholder".to_string(),
            category: TagCategory::Audience,
            instruction_body: "Write as if addressing an external partner or stakeholder."
                .to_string(),
            is_built_in: true,
            balancing_group: Some("audience".to_string()),
        },
        BuiltInTag {
            id: "builtin-mixed-audience".to_string(),
            name: "Mixed audience".to_string(),
            category: TagCategory::Audience,
            instruction_body: "Write for a mixed audience of varying seniority and familiarity."
                .to_string(),
            is_built_in: true,
            balancing_group: Some("audience".to_string()),
        },
        // --- Tone (multi-select, no balancing group) ---
        BuiltInTag {
            id: "builtin-professional".to_string(),
            name: "Professional".to_string(),
            category: TagCategory::Tone,
            instruction_body: "Use professional, workplace-appropriate language.".to_string(),
            is_built_in: true,
            balancing_group: None,
        },
        BuiltInTag {
            id: "builtin-polite".to_string(),
            name: "Polite".to_string(),
            category: TagCategory::Tone,
            instruction_body: "Use polite, courteous language.".to_string(),
            is_built_in: true,
            balancing_group: None,
        },
        BuiltInTag {
            id: "builtin-friendly".to_string(),
            name: "Friendly".to_string(),
            category: TagCategory::Tone,
            instruction_body: "Adopt a warm, friendly tone.".to_string(),
            is_built_in: true,
            balancing_group: None,
        },
        BuiltInTag {
            id: "builtin-objective".to_string(),
            name: "Objective".to_string(),
            category: TagCategory::Tone,
            instruction_body: "Maintain an objective, fact-based tone without personal opinion."
                .to_string(),
            is_built_in: true,
            balancing_group: None,
        },
        BuiltInTag {
            id: "builtin-enthusiastic".to_string(),
            name: "Enthusiastic".to_string(),
            category: TagCategory::Tone,
            instruction_body: "Use an enthusiastic, energetic tone.".to_string(),
            is_built_in: true,
            balancing_group: None,
        },
        BuiltInTag {
            id: "builtin-calm".to_string(),
            name: "Calm".to_string(),
            category: TagCategory::Tone,
            instruction_body: "Use a calm, measured tone.".to_string(),
            is_built_in: true,
            balancing_group: None,
        },
        BuiltInTag {
            id: "builtin-empathetic".to_string(),
            name: "Empathetic".to_string(),
            category: TagCategory::Tone,
            instruction_body: "Use an empathetic, understanding tone.".to_string(),
            is_built_in: true,
            balancing_group: None,
        },
        // --- Format ---
        BuiltInTag {
            id: "builtin-email".to_string(),
            name: "Email".to_string(),
            category: TagCategory::Format,
            instruction_body:
                "Format the output as a professional email with greeting and sign-off.".to_string(),
            is_built_in: true,
            balancing_group: Some("format".to_string()),
        },
        BuiltInTag {
            id: "builtin-direct-message".to_string(),
            name: "Direct message".to_string(),
            category: TagCategory::Format,
            instruction_body: "Format as a concise direct message (e.g. Slack or Teams)."
                .to_string(),
            is_built_in: true,
            balancing_group: Some("format".to_string()),
        },
        BuiltInTag {
            id: "builtin-group-message".to_string(),
            name: "Group message".to_string(),
            category: TagCategory::Format,
            instruction_body: "Format as a message suitable for a group channel or thread."
                .to_string(),
            is_built_in: true,
            balancing_group: Some("format".to_string()),
        },
        BuiltInTag {
            id: "builtin-paragraph".to_string(),
            name: "Paragraph".to_string(),
            category: TagCategory::Format,
            instruction_body: "Structure the output as flowing paragraphs.".to_string(),
            is_built_in: true,
            balancing_group: Some("format".to_string()),
        },
        BuiltInTag {
            id: "builtin-bullet-points".to_string(),
            name: "Bullet points".to_string(),
            category: TagCategory::Format,
            instruction_body: "Structure the output as bullet points.".to_string(),
            is_built_in: true,
            balancing_group: Some("format".to_string()),
        },
        // --- Clarity ---
        BuiltInTag {
            id: "builtin-simplify".to_string(),
            name: "Simplify".to_string(),
            category: TagCategory::Clarity,
            instruction_body: "Simplify the language for easier understanding.".to_string(),
            is_built_in: true,
            balancing_group: Some("clarity".to_string()),
        },
        BuiltInTag {
            id: "builtin-clarify".to_string(),
            name: "Clarify".to_string(),
            category: TagCategory::Clarity,
            instruction_body: "Clarify ambiguous or unclear phrasing.".to_string(),
            is_built_in: true,
            balancing_group: Some("clarity".to_string()),
        },
        BuiltInTag {
            id: "builtin-more-precise".to_string(),
            name: "More precise".to_string(),
            category: TagCategory::Clarity,
            instruction_body: "Use more precise, specific language.".to_string(),
            is_built_in: true,
            balancing_group: Some("clarity".to_string()),
        },
        BuiltInTag {
            id: "builtin-presentation-ready".to_string(),
            name: "Presentation-ready".to_string(),
            category: TagCategory::Clarity,
            instruction_body: "Polish the text to be presentation-ready and publication-quality."
                .to_string(),
            is_built_in: true,
            balancing_group: Some("clarity".to_string()),
        },
        // --- Length ---
        BuiltInTag {
            id: "builtin-shorter".to_string(),
            name: "Shorter".to_string(),
            category: TagCategory::Length,
            instruction_body: "Make the text more concise.".to_string(),
            is_built_in: true,
            balancing_group: Some("length".to_string()),
        },
        BuiltInTag {
            id: "builtin-same-length".to_string(),
            name: "Same length".to_string(),
            category: TagCategory::Length,
            instruction_body: "Keep the output roughly the same length as the input.".to_string(),
            is_built_in: true,
            balancing_group: Some("length".to_string()),
        },
        BuiltInTag {
            id: "builtin-longer".to_string(),
            name: "Longer".to_string(),
            category: TagCategory::Length,
            instruction_body: "Expand the text with additional detail.".to_string(),
            is_built_in: true,
            balancing_group: Some("length".to_string()),
        },
        // --- Directness ---
        BuiltInTag {
            id: "builtin-more-direct".to_string(),
            name: "More direct".to_string(),
            category: TagCategory::Directness,
            instruction_body: "Use more direct, assertive language.".to_string(),
            is_built_in: true,
            balancing_group: Some("directness".to_string()),
        },
        BuiltInTag {
            id: "builtin-more-diplomatic".to_string(),
            name: "More diplomatic".to_string(),
            category: TagCategory::Directness,
            instruction_body: "Soften the language to be more diplomatic and tactful.".to_string(),
            is_built_in: true,
            balancing_group: Some("directness".to_string()),
        },
        // --- Technicality ---
        BuiltInTag {
            id: "builtin-technical".to_string(),
            name: "Technical".to_string(),
            category: TagCategory::Technicality,
            instruction_body: "Use precise technical terminology.".to_string(),
            is_built_in: true,
            balancing_group: Some("technicality".to_string()),
        },
        BuiltInTag {
            id: "builtin-non-technical".to_string(),
            name: "Non-technical".to_string(),
            category: TagCategory::Technicality,
            instruction_body: "Avoid technical jargon; use plain language.".to_string(),
            is_built_in: true,
            balancing_group: Some("technicality".to_string()),
        },
    ]
}
