// Phase: 6
// Tag panel — displays built-in and custom tags grouped by category

import { useState } from "react";
import { useAppStore } from "../../state/store";
import { TagChip } from "./TagChip";
import { TagCategorySection } from "./TagCategorySection";
import { TagConflictHint } from "./TagConflictHint";
import { CustomTagEditorModal } from "./CustomTagEditorModal";
import type { TagCategory } from "../../ipc/types";

const CATEGORY_ORDER: {
  key: TagCategory;
  displayName: string;
  defaultExpanded: boolean;
}[] = [
  { key: "audience", displayName: "Audience", defaultExpanded: true },
  { key: "tone", displayName: "Tone", defaultExpanded: true },
  { key: "format", displayName: "Format", defaultExpanded: true },
  { key: "clarity", displayName: "Clarity", defaultExpanded: false },
  { key: "length", displayName: "Length", defaultExpanded: false },
  { key: "directness", displayName: "Directness", defaultExpanded: false },
  { key: "technicality", displayName: "Technicality", defaultExpanded: false },
];

export function TagPanel() {
  const builtInTags = useAppStore((s) => s.metadata.builtInTags);
  const customTags = useAppStore((s) => s.metadata.customTags);
  const loadStatus = useAppStore((s) => s.metadata.loadStatus);
  const activeTab = useAppStore((s) =>
    s.tabs.find((t) => t.id === s.activeTabId),
  );
  const toggleTag = useAppStore((s) => s.toggleTag);

  const [editorTagId, setEditorTagId] = useState<string | null | undefined>(
    undefined,
  );
  // undefined = modal closed, null = create mode, string = edit mode

  if (!activeTab) return null;

  const activeTagIds = activeTab.activeTagIds;

  function handleToggle(tagId: string) {
    if (activeTab) {
      toggleTag(activeTab.id, tagId);
    }
  }

  // Group built-in tags by category
  const tagsByCategory = new Map<TagCategory, typeof builtInTags>();
  for (const tag of builtInTags) {
    const group = tagsByCategory.get(tag.category);
    if (group) {
      group.push(tag);
    } else {
      tagsByCategory.set(tag.category, [tag]);
    }
  }

  return (
    <div className="tag-panel">
      <div className="tag-panel-header">
        <h3>Tags</h3>
        {activeTagIds.length > 0 && (
          <span className="tag-panel-count">{activeTagIds.length} active</span>
        )}
      </div>

      {loadStatus !== "loaded" && loadStatus !== "idle" && (
        <p className="tag-panel-loading">Loading tags...</p>
      )}

      {/* Built-in tag categories */}
      {CATEGORY_ORDER.map(({ key, displayName, defaultExpanded }) => {
        const tags = tagsByCategory.get(key);
        if (!tags || tags.length === 0) return null;
        return (
          <TagCategorySection
            key={key}
            category={key}
            displayName={displayName}
            defaultExpanded={defaultExpanded}
          >
            {tags.map((tag) => (
              <TagChip
                key={tag.id}
                id={tag.id}
                name={tag.name}
                active={activeTagIds.includes(tag.id)}
                instructionBody={tag.instructionBody}
                isCustom={false}
                onToggle={handleToggle}
              />
            ))}
          </TagCategorySection>
        );
      })}

      {/* Custom tags section */}
      <TagCategorySection
        category="custom"
        displayName="Custom"
        defaultExpanded={true}
      >
        {customTags.length === 0 ? (
          <p className="tag-empty-state">No custom tags yet.</p>
        ) : (
          customTags.map((tag) => (
            <TagChip
              key={tag.id}
              id={tag.id}
              name={tag.name}
              active={activeTagIds.includes(tag.id)}
              instructionBody={tag.instructionBody}
              isCustom={true}
              onToggle={handleToggle}
              onEdit={(id) => setEditorTagId(id)}
            />
          ))
        )}
        <button
          className="tag-add-btn"
          onClick={() => setEditorTagId(null)}
          data-testid="add-custom-tag-btn"
        >
          + Add Tag
        </button>
      </TagCategorySection>

      <TagConflictHint
        activeTagIds={activeTagIds}
        builtInTags={builtInTags}
        customTags={customTags}
      />

      {/* Custom tag editor modal */}
      {editorTagId !== undefined && (
        <CustomTagEditorModal
          tagId={editorTagId}
          onClose={() => setEditorTagId(undefined)}
        />
      )}
    </div>
  );
}
