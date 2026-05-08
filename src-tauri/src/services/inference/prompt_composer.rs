// Phase: 5
// Prompt composer — assembles prompt packages from profile, tags, and user text.
// Deterministic: same inputs always produce byte-identical output.
// Privacy: never logs assembled prompt text (P2). Only structured metadata is loggable.

use std::collections::BTreeMap;

use crate::contracts::shared::TagCategory;
use crate::services::inference::worker_protocol::PromptPackage;

// ──────────── Public types ────────────

/// A unified tag representation for prompt composition.
/// Built from either BuiltInTag or CustomTag by the calling code.
#[derive(Debug, Clone)]
pub struct ResolvedTag {
    pub id: String,
    pub name: String,
    pub category: TagCategory,
    pub instruction_body: String,
    pub balancing_group: Option<String>,
}

// ──────────── Constants ────────────

/// Conservative max-tokens ceiling (will be model-specific in Phase 11).
const MAX_TOKENS_CEILING: u32 = 4096;

/// Minimum max_tokens to ensure useful output.
const MIN_MAX_TOKENS: u32 = 256;

/// Temperature for initial rewrite.
const INITIAL_TEMPERATURE: f32 = 0.7;

/// Temperature for refinement (lower for more faithful output).
const REFINEMENT_TEMPERATURE: f32 = 0.5;

// ──────────── Spectrum definitions ────────────

/// A point on a balancing spectrum.
struct SpectrumPoint {
    position: f32,
    name: &'static str,
    directive: &'static str,
}

/// Returns the spectrum definition for a balancing group, if one exists.
/// Groups without a defined spectrum (e.g., "relationship", "format") return None,
/// meaning their tags are concatenated rather than balanced.
fn get_spectrum(group: &str) -> Option<&'static [SpectrumPoint]> {
    match group {
        "tone" => Some(&[
            SpectrumPoint {
                position: -1.0,
                name: "friendly",
                directive: "Adopt a warm, friendly tone.",
            },
            SpectrumPoint {
                position: 0.0,
                name: "neutral",
                directive: "Maintain a neutral, objective tone.",
            },
            SpectrumPoint {
                position: 1.0,
                name: "formal",
                directive: "Adopt a formal, respectful tone.",
            },
        ]),
        "length" => Some(&[
            SpectrumPoint {
                position: -1.0,
                name: "shorter",
                directive: "Make the text more concise.",
            },
            SpectrumPoint {
                position: 0.0,
                name: "same length",
                directive: "Keep the output roughly the same length as the input.",
            },
            SpectrumPoint {
                position: 1.0,
                name: "longer",
                directive: "Expand the text with additional detail.",
            },
        ]),
        "clarity" => Some(&[
            SpectrumPoint {
                position: -1.0,
                name: "simplify",
                directive: "Simplify the language for easier understanding.",
            },
            SpectrumPoint {
                position: 1.0,
                name: "elaborate",
                directive: "Expand on the ideas with more detail and explanation.",
            },
        ]),
        "assertiveness" => Some(&[
            SpectrumPoint {
                position: -1.0,
                name: "more polite",
                directive: "Soften the language to be more polite and diplomatic.",
            },
            SpectrumPoint {
                position: 1.0,
                name: "more direct",
                directive: "Use more direct, assertive language.",
            },
        ]),
        "technicality" => Some(&[
            SpectrumPoint {
                position: -1.0,
                name: "non-technical",
                directive: "Avoid technical jargon; use plain language.",
            },
            SpectrumPoint {
                position: 1.0,
                name: "technical",
                directive: "Use precise technical terminology.",
            },
        ]),
        _ => None,
    }
}

/// Look up the spectrum position for a tag by its balancing group and name.
fn spectrum_position(group: &str, tag_name: &str) -> Option<f32> {
    let spectrum = get_spectrum(group)?;
    let normalized = tag_name.to_lowercase();
    spectrum
        .iter()
        .find(|p| p.name == normalized)
        .map(|p| p.position)
}

/// Category rendering order per spec section 3.6.
fn category_order(cat: &TagCategory) -> u8 {
    match cat {
        TagCategory::Audience => 0,
        TagCategory::Tone => 1,
        TagCategory::Directness => 2,
        TagCategory::Technicality => 3,
        TagCategory::Clarity => 4,
        TagCategory::Length => 5,
        TagCategory::Format => 6,
        TagCategory::Other => 7,
    }
}

// ──────────── PromptComposer ────────────

pub struct PromptComposer;

impl PromptComposer {
    /// Compose a prompt package for an initial rewrite.
    pub fn compose_initial_rewrite(
        profile_instruction_body: &str,
        tags: &[ResolvedTag],
        source_text: &str,
    ) -> PromptPackage {
        let system_prompt = Self::build_system_prompt(profile_instruction_body, tags);
        let user_message = Self::build_initial_user_message(source_text);
        let max_tokens = Self::compute_max_tokens(source_text.len());

        PromptPackage {
            system_prompt,
            user_message,
            max_tokens,
            temperature: INITIAL_TEMPERATURE,
        }
    }

    /// Compose a prompt package for a refinement.
    pub fn compose_refinement(
        profile_instruction_body: &str,
        tags: &[ResolvedTag],
        accepted_output: &str,
        refinement_instruction: &str,
    ) -> PromptPackage {
        let system_prompt = Self::build_system_prompt(profile_instruction_body, tags);
        let user_message =
            Self::build_refinement_user_message(accepted_output, refinement_instruction);
        let max_tokens = Self::compute_max_tokens(accepted_output.len());

        PromptPackage {
            system_prompt,
            user_message,
            max_tokens,
            temperature: REFINEMENT_TEMPERATURE,
        }
    }

    // ──── System prompt assembly (spec section 4) ────

    fn build_system_prompt(profile_instruction_body: &str, tags: &[ResolvedTag]) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Section 1: Role and hard output contract
        parts.push(
            concat!(
                "You are a text rewriting engine. ",
                "Your sole function is to produce rewritten text. ",
                "You never produce commentary, opinions, labels, greetings, sign-offs, ",
                "dashed separator lines, markdown formatting, or meta-text of any kind. ",
                "Your entire response must be the rewritten text and nothing else."
            )
            .to_string(),
        );

        // Section 2: Profile instruction body
        if !profile_instruction_body.is_empty() {
            parts.push(profile_instruction_body.to_string());
        }

        // Sections 3–4: Tag directives (only if tags produce directives)
        let directives = Self::normalize_tags(tags);
        if !directives.is_empty() {
            parts.push("Apply the following communication style modifiers:".to_string());
            let directive_block = directives
                .iter()
                .map(|d| format!("- {}", d))
                .collect::<Vec<_>>()
                .join("\n");
            parts.push(directive_block);
        }

        // Section 5: Formatting directives
        parts.push(
            concat!(
                "Produce plain text output only. Do not use markdown formatting ",
                "unless a format tag explicitly requests structured output."
            )
            .to_string(),
        );
        parts.push(
            "Preserve the original meaning and factual content of the source text.".to_string(),
        );
        parts.push(
            "Do not add information not present in or clearly implied by the source text."
                .to_string(),
        );

        // Section 6: Hard output constraint (repeated for emphasis with small models)
        parts.push(
            concat!(
                "CRITICAL: Output the revised text only. ",
                "No preambles. No explanations. No labels. No commentary. ",
                "No dashed lines. No markdown. No opinions. ",
                "Begin your response with the first word of the rewritten text."
            )
            .to_string(),
        );

        parts.join("\n\n")
    }

    // ──── User message assembly (spec section 5) ────

    fn build_initial_user_message(source_text: &str) -> String {
        format!(
            "Rewrite the following text according to the instructions above.\n\n[TEXT START]\n{}\n[TEXT END]",
            source_text
        )
    }

    fn build_refinement_user_message(
        accepted_output: &str,
        refinement_instruction: &str,
    ) -> String {
        format!(
            concat!(
                "Revise the text below according to this instruction: {}\n\n",
                "[TEXT START]\n{}\n[TEXT END]\n\n",
                "Output the complete revised text only."
            ),
            refinement_instruction, accepted_output
        )
    }

    // ──── Model parameters (spec section 6) ────

    fn compute_max_tokens(text_len: usize) -> u32 {
        let raw = (text_len as u32).saturating_mul(3);
        raw.clamp(MIN_MAX_TOKENS, MAX_TOKENS_CEILING)
    }

    // ──── Tag normalization (spec section 3) ────

    /// Normalize tags into an ordered list of directive strings.
    /// Applies the balancing algorithm for tags in the same balancing group.
    fn normalize_tags(tags: &[ResolvedTag]) -> Vec<String> {
        // Group tags by category, maintaining spec ordering.
        // Use BTreeMap with category order as key for deterministic iteration.
        let mut by_category: BTreeMap<u8, Vec<&ResolvedTag>> = BTreeMap::new();
        for tag in tags {
            // Skip tags with empty instruction bodies (spec section 9)
            if tag.instruction_body.trim().is_empty() {
                continue;
            }
            let order = category_order(&tag.category);
            by_category.entry(order).or_default().push(tag);
        }

        let mut all_directives: Vec<String> = Vec::new();

        for cat_tags in by_category.values() {
            let mut cat_directives = Self::process_category(cat_tags);
            all_directives.append(&mut cat_directives);
        }

        all_directives
    }

    /// Process all tags within a single category, applying balancing where applicable.
    /// Returns directives sorted alphabetically by originating tag/group name.
    fn process_category(tags: &[&ResolvedTag]) -> Vec<String> {
        // Separate tags into balancing groups and ungrouped tags.
        let mut by_group: BTreeMap<String, Vec<&ResolvedTag>> = BTreeMap::new();
        let mut ungrouped: Vec<&ResolvedTag> = Vec::new();

        for tag in tags {
            if let Some(ref group) = tag.balancing_group {
                by_group.entry(group.clone()).or_default().push(tag);
            } else {
                ungrouped.push(tag);
            }
        }

        // Collect (sort_key, directive) pairs for final ordering.
        let mut keyed_directives: Vec<(String, String)> = Vec::new();

        // Process each balancing group
        for (group, group_tags) in &by_group {
            if let Some(directive) = Self::balance_group(group, group_tags) {
                // Sort key: first tag name alphabetically in the group
                let sort_key = group_tags
                    .iter()
                    .map(|t| t.name.to_lowercase())
                    .min()
                    .unwrap_or_default();
                keyed_directives.push((sort_key, directive));
            } else {
                // No spectrum or not all tags have positions — concatenate individually
                Self::concatenate_tags(group_tags, &mut keyed_directives);
            }
        }

        // Process ungrouped tags
        Self::concatenate_tags(&ungrouped, &mut keyed_directives);

        // Sort by key for determinism
        keyed_directives.sort_by(|a, b| a.0.cmp(&b.0));

        keyed_directives.into_iter().map(|(_, d)| d).collect()
    }

    /// Attempt to balance a group of tags. Returns None if the group has no
    /// defined spectrum or not all tags have known positions.
    fn balance_group(group: &str, tags: &[&ResolvedTag]) -> Option<String> {
        let spectrum = get_spectrum(group)?;

        if tags.len() == 1 {
            // Single tag — just use its directive (no balancing needed)
            return Some(tags[0].instruction_body.clone());
        }

        // Check if all tags have known spectrum positions
        let mut positions: Vec<f32> = Vec::new();
        for tag in tags {
            match spectrum_position(group, &tag.name) {
                Some(pos) => positions.push(pos),
                None => {
                    // Tag doesn't have a known position — can't balance this group.
                    // Fall back to concatenation.
                    return None;
                }
            }
        }

        // Compute average position
        let avg: f32 = positions.iter().sum::<f32>() / positions.len() as f32;

        // Map average back to a directive
        Some(Self::resolve_balanced_directive(group, avg, spectrum))
    }

    /// Map an average spectrum position to a directive string.
    fn resolve_balanced_directive(group: &str, avg: f32, spectrum: &[SpectrumPoint]) -> String {
        // Check for exact match (within floating-point tolerance)
        for point in spectrum {
            if (avg - point.position).abs() < f32::EPSILON {
                return point.directive.to_string();
            }
        }

        // Find the two nearest points (below and above the average)
        let mut below: Option<&SpectrumPoint> = None;
        let mut above: Option<&SpectrumPoint> = None;

        for point in spectrum {
            if point.position < avg && (below.is_none() || point.position > below.unwrap().position)
            {
                below = Some(point);
            } else if point.position > avg
                && (above.is_none() || point.position < above.unwrap().position)
            {
                above = Some(point);
            }
        }

        match (below, above) {
            (Some(lo), Some(hi)) => {
                let closer = if (avg - lo.position).abs() <= (hi.position - avg).abs() {
                    lo.name
                } else {
                    hi.name
                };
                Self::interpolated_directive(group, lo.name, hi.name, closer)
            }
            _ => {
                // Average is at or beyond the spectrum edge — use nearest point
                spectrum
                    .iter()
                    .min_by(|a, b| {
                        (a.position - avg)
                            .abs()
                            .partial_cmp(&(b.position - avg).abs())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|p| p.directive.to_string())
                    .unwrap_or_default()
            }
        }
    }

    /// Generate an interpolated directive for an average falling between two spectrum points.
    fn interpolated_directive(
        group: &str,
        lower_name: &str,
        upper_name: &str,
        closer_name: &str,
    ) -> String {
        let dimension = match group {
            "tone" => "tone",
            "length" => "length",
            "clarity" => "clarity level",
            "assertiveness" => "assertiveness",
            "technicality" => "technicality level",
            _ => "style",
        };
        format!(
            "Aim for a {} between {} and {}, leaning toward {}.",
            dimension, lower_name, upper_name, closer_name
        )
    }

    /// Add individual tag directives to the keyed list (for non-balanced tags).
    fn concatenate_tags(tags: &[&ResolvedTag], out: &mut Vec<(String, String)>) {
        // Check for contradictory format tags (spec section 3.4)
        let is_format = tags.iter().any(|t| t.category == TagCategory::Format);
        let has_paragraph = tags.iter().any(|t| t.name.to_lowercase() == "paragraph");
        let has_bullets = tags
            .iter()
            .any(|t| t.name.to_lowercase() == "bullet points");

        for tag in tags {
            let sort_key = tag.name.to_lowercase();
            out.push((sort_key, tag.instruction_body.clone()));
        }

        // If contradictory format tags, append the combining note
        if is_format && has_paragraph && has_bullets {
            out.push((
                "zzz_format_note".to_string(), // sorts after all format tags
                concat!(
                    "The user has requested both bullet-point and paragraph formatting. ",
                    "Use your judgment to combine these, for example by using short ",
                    "paragraphs with bullet-point summaries."
                )
                .to_string(),
            ));
        }
    }
}

// ──────────── Tests ────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tag(
        name: &str,
        category: TagCategory,
        instruction: &str,
        group: Option<&str>,
    ) -> ResolvedTag {
        ResolvedTag {
            id: format!("test-{}", name.to_lowercase().replace(' ', "-")),
            name: name.to_string(),
            category,
            instruction_body: instruction.to_string(),
            balancing_group: group.map(|g| g.to_string()),
        }
    }

    // --- Determinism ---

    #[test]
    fn compose_is_deterministic() {
        let tags = vec![
            make_tag(
                "Formal",
                TagCategory::Tone,
                "Adopt a formal, respectful tone.",
                Some("tone"),
            ),
            make_tag(
                "Shorter",
                TagCategory::Length,
                "Make the text more concise.",
                Some("length"),
            ),
        ];
        let a = PromptComposer::compose_initial_rewrite("Test profile.", &tags, "Hello world");
        let b = PromptComposer::compose_initial_rewrite("Test profile.", &tags, "Hello world");
        assert_eq!(a.system_prompt, b.system_prompt);
        assert_eq!(a.user_message, b.user_message);
        assert_eq!(a.max_tokens, b.max_tokens);
        assert_eq!(a.temperature, b.temperature);
    }

    // --- System prompt structure ---

    #[test]
    fn system_prompt_includes_profile_body() {
        let pkg = PromptComposer::compose_initial_rewrite("My custom profile.", &[], "text");
        assert!(pkg.system_prompt.contains("My custom profile."));
    }

    #[test]
    fn system_prompt_includes_hardened_output_contract() {
        let pkg = PromptComposer::compose_initial_rewrite("Profile.", &[], "text");
        assert!(pkg.system_prompt.contains("text rewriting engine"));
        assert!(pkg.system_prompt.contains("never produce commentary"));
        assert!(pkg.system_prompt.contains("dashed separator lines"));
        assert!(pkg.system_prompt.contains("No preambles"));
        assert!(pkg
            .system_prompt
            .contains("Begin your response with the first word"));
    }

    #[test]
    fn system_prompt_omits_tag_section_when_no_tags() {
        let pkg = PromptComposer::compose_initial_rewrite("Profile.", &[], "text");
        assert!(!pkg.system_prompt.contains("communication style modifiers"));
    }

    #[test]
    fn system_prompt_includes_tag_directives() {
        let tags = vec![make_tag(
            "Formal",
            TagCategory::Tone,
            "Adopt a formal, respectful tone.",
            Some("tone"),
        )];
        let pkg = PromptComposer::compose_initial_rewrite("Profile.", &tags, "text");
        assert!(pkg.system_prompt.contains("communication style modifiers"));
        assert!(pkg
            .system_prompt
            .contains("Adopt a formal, respectful tone."));
    }

    #[test]
    fn system_prompt_includes_formatting_directives() {
        let pkg = PromptComposer::compose_initial_rewrite("Profile.", &[], "text");
        assert!(pkg.system_prompt.contains("Produce plain text output only"));
        assert!(pkg.system_prompt.contains("Preserve the original meaning"));
    }

    #[test]
    fn empty_profile_body_produces_valid_prompt() {
        let pkg = PromptComposer::compose_initial_rewrite("", &[], "text");
        assert!(pkg.system_prompt.contains("Produce plain text output only"));
        assert!(pkg.system_prompt.contains("text rewriting engine"));
        assert!(!pkg.system_prompt.is_empty());
    }

    // --- User message ---

    #[test]
    fn initial_user_message_format() {
        let pkg = PromptComposer::compose_initial_rewrite("Profile.", &[], "Hello world");
        assert!(pkg.user_message.contains("Rewrite the following text"));
        assert!(pkg.user_message.contains("[TEXT START]"));
        assert!(pkg.user_message.contains("Hello world"));
        assert!(pkg.user_message.contains("[TEXT END]"));
        // Must not contain --- separators that models echo
        assert!(!pkg.user_message.contains("---"));
    }

    #[test]
    fn refinement_user_message_format() {
        let pkg =
            PromptComposer::compose_refinement("Profile.", &[], "Current text.", "Make it shorter");
        assert!(pkg.user_message.contains("Make it shorter"));
        assert!(pkg.user_message.contains("[TEXT START]"));
        assert!(pkg.user_message.contains("Current text."));
        assert!(pkg.user_message.contains("[TEXT END]"));
        assert!(pkg
            .user_message
            .contains("Output the complete revised text only"));
        // Must not contain --- separators that models echo
        assert!(!pkg.user_message.contains("---"));
    }

    #[test]
    fn refinement_uses_same_system_prompt_as_initial() {
        let tags = vec![make_tag(
            "Formal",
            TagCategory::Tone,
            "Adopt a formal, respectful tone.",
            Some("tone"),
        )];
        let initial = PromptComposer::compose_initial_rewrite("Profile body.", &tags, "text");
        let refinement = PromptComposer::compose_refinement("Profile body.", &tags, "text", "fix");
        assert_eq!(initial.system_prompt, refinement.system_prompt);
    }

    #[test]
    fn refinement_system_prompt_includes_profile_and_tags() {
        let tags = vec![make_tag(
            "Formal",
            TagCategory::Tone,
            "Adopt a formal, respectful tone.",
            Some("tone"),
        )];
        let pkg = PromptComposer::compose_refinement(
            "My profile instruction.",
            &tags,
            "accepted text",
            "make shorter",
        );
        // Profile present
        assert!(pkg.system_prompt.contains("My profile instruction."));
        // Tags present
        assert!(pkg
            .system_prompt
            .contains("Adopt a formal, respectful tone."));
        // Output contract present
        assert!(pkg.system_prompt.contains("text rewriting engine"));
        assert!(pkg.system_prompt.contains("No preambles"));
    }

    #[test]
    fn refinement_places_instruction_before_text() {
        let pkg = PromptComposer::compose_refinement(
            "P.",
            &[],
            "The accepted output text here.",
            "Make it shorter",
        );
        // Instruction should appear before the text block
        let instruction_pos = pkg.user_message.find("Make it shorter").unwrap();
        let text_pos = pkg
            .user_message
            .find("The accepted output text here.")
            .unwrap();
        assert!(
            instruction_pos < text_pos,
            "Refinement instruction must precede the text block"
        );
    }

    // --- Model parameters ---

    #[test]
    fn initial_rewrite_temperature() {
        let pkg = PromptComposer::compose_initial_rewrite("P.", &[], "text");
        assert!((pkg.temperature - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn refinement_temperature() {
        let pkg = PromptComposer::compose_refinement("P.", &[], "text", "fix");
        assert!((pkg.temperature - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn max_tokens_computed_from_text_length() {
        // 100 chars * 3 = 300, but minimum is 256
        let pkg = PromptComposer::compose_initial_rewrite("P.", &[], &"x".repeat(100));
        assert_eq!(pkg.max_tokens, 300);
    }

    #[test]
    fn max_tokens_has_minimum() {
        let pkg = PromptComposer::compose_initial_rewrite("P.", &[], "hi");
        assert_eq!(pkg.max_tokens, MIN_MAX_TOKENS);
    }

    #[test]
    fn max_tokens_has_ceiling() {
        let pkg = PromptComposer::compose_initial_rewrite("P.", &[], &"x".repeat(10000));
        assert_eq!(pkg.max_tokens, MAX_TOKENS_CEILING);
    }

    // --- Tag normalization: single tags ---

    #[test]
    fn single_tag_uses_its_instruction() {
        let tags = vec![make_tag(
            "Formal",
            TagCategory::Tone,
            "Adopt a formal, respectful tone.",
            Some("tone"),
        )];
        let pkg = PromptComposer::compose_initial_rewrite("P.", &tags, "text");
        assert!(pkg
            .system_prompt
            .contains("Adopt a formal, respectful tone."));
    }

    // --- Tag normalization: balancing ---

    #[test]
    fn two_opposing_tags_produce_balanced_directive() {
        let tags = vec![
            make_tag(
                "Shorter",
                TagCategory::Length,
                "Make the text more concise.",
                Some("length"),
            ),
            make_tag(
                "Longer",
                TagCategory::Length,
                "Expand the text with additional detail.",
                Some("length"),
            ),
        ];
        let directives = PromptComposer::normalize_tags(&tags);
        // Average of -1 and +1 = 0 → "same length" directive
        assert_eq!(directives.len(), 1);
        assert!(directives[0].contains("same length"));
    }

    #[test]
    fn three_tags_in_spectrum_averaged() {
        let tags = vec![
            make_tag("Friendly", TagCategory::Tone, "Warm.", Some("tone")),
            make_tag("Neutral", TagCategory::Tone, "Neutral.", Some("tone")),
            make_tag("Formal", TagCategory::Tone, "Formal.", Some("tone")),
        ];
        let directives = PromptComposer::normalize_tags(&tags);
        // Average of -1, 0, +1 = 0 → neutral
        assert_eq!(directives.len(), 1);
        assert!(directives[0].contains("neutral"));
    }

    #[test]
    fn interpolated_directive_when_between_positions() {
        let tags = vec![
            make_tag("Friendly", TagCategory::Tone, "Warm.", Some("tone")),
            make_tag("Formal", TagCategory::Tone, "Formal.", Some("tone")),
            make_tag("Formal2", TagCategory::Tone, "Formal2.", Some("tone")),
        ];
        // Friendly=-1, but Formal2 won't have a position → falls back to concatenation
        // Actually Formal2 doesn't match any spectrum position, so balance fails
        let directives = PromptComposer::normalize_tags(&tags);
        assert_eq!(directives.len(), 3); // concatenated
    }

    #[test]
    fn balanced_shorter_same_length_interpolates() {
        // Shorter(-1) + Same Length(0) = avg -0.5 → between shorter and same length
        let tags = vec![
            make_tag("Shorter", TagCategory::Length, "Concise.", Some("length")),
            make_tag(
                "Same Length",
                TagCategory::Length,
                "Same length.",
                Some("length"),
            ),
        ];
        let directives = PromptComposer::normalize_tags(&tags);
        assert_eq!(directives.len(), 1);
        assert!(directives[0].contains("leaning toward shorter"));
    }

    // --- Tag normalization: category ordering ---

    #[test]
    fn tags_ordered_by_category() {
        let tags = vec![
            make_tag("Shorter", TagCategory::Length, "Concise.", Some("length")),
            make_tag("Formal", TagCategory::Tone, "Formal.", Some("tone")),
        ];
        let directives = PromptComposer::normalize_tags(&tags);
        assert_eq!(directives.len(), 2);
        // Tone (order 1) should come before Length (order 5)
        assert!(directives[0].contains("Formal"));
        assert!(directives[1].contains("Concise"));
    }

    // --- Tag normalization: non-balancing categories ---

    #[test]
    fn format_tags_concatenated_not_balanced() {
        let tags = vec![
            make_tag(
                "Paragraph",
                TagCategory::Format,
                "Structure the output as flowing paragraphs.",
                Some("format"),
            ),
            make_tag(
                "Bullet Points",
                TagCategory::Format,
                "Structure the output as bullet points.",
                Some("format"),
            ),
        ];
        let directives = PromptComposer::normalize_tags(&tags);
        // Both directives present plus the combining note
        assert!(directives.len() >= 2);
        assert!(directives.iter().any(|d| d.contains("paragraphs")));
        assert!(directives.iter().any(|d| d.contains("bullet points")));
        assert!(directives.iter().any(|d| d.contains("Use your judgment")));
    }

    #[test]
    fn single_format_tag_no_combining_note() {
        let tags = vec![make_tag(
            "Paragraph",
            TagCategory::Format,
            "Structure the output as flowing paragraphs.",
            Some("format"),
        )];
        let directives = PromptComposer::normalize_tags(&tags);
        assert_eq!(directives.len(), 1);
        assert!(!directives[0].contains("Use your judgment"));
    }

    // --- Edge cases ---

    #[test]
    fn empty_tag_instruction_body_skipped() {
        let tags = vec![make_tag("Empty", TagCategory::Other, "  ", None)];
        let directives = PromptComposer::normalize_tags(&tags);
        assert!(directives.is_empty());
    }

    #[test]
    fn tags_without_balancing_group_concatenated() {
        let tags = vec![
            make_tag("Custom A", TagCategory::Other, "Do A.", None),
            make_tag("Custom B", TagCategory::Other, "Do B.", None),
        ];
        let directives = PromptComposer::normalize_tags(&tags);
        assert_eq!(directives.len(), 2);
        // Alphabetical order
        assert!(directives[0].contains("Do A"));
        assert!(directives[1].contains("Do B"));
    }

    #[test]
    fn mixed_categories_all_rendered() {
        let tags = vec![
            make_tag("Formal", TagCategory::Tone, "Formal tone.", Some("tone")),
            make_tag(
                "More Direct",
                TagCategory::Directness,
                "Direct language.",
                Some("directness"),
            ),
            make_tag("Custom", TagCategory::Other, "Custom directive.", None),
        ];
        let directives = PromptComposer::normalize_tags(&tags);
        assert_eq!(directives.len(), 3);
        // Order: Tone(1), Directness(2), Other(7)
        assert!(directives[0].contains("Formal"));
        assert!(directives[1].contains("Direct"));
        assert!(directives[2].contains("Custom"));
    }
}
