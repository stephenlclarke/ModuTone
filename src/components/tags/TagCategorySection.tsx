// Phase: 6

import { useState, type ReactNode } from "react";

interface TagCategorySectionProps {
  category: string;
  displayName: string;
  children: ReactNode;
  defaultExpanded?: boolean;
}

export function TagCategorySection({
  category,
  displayName,
  children,
  defaultExpanded = true,
}: TagCategorySectionProps) {
  const [expanded, setExpanded] = useState(defaultExpanded);

  return (
    <div className="tag-category-section" data-category={category}>
      <button
        className="tag-category-header"
        onClick={() => setExpanded((prev) => !prev)}
        aria-expanded={expanded}
      >
        <span className="tag-category-collapse-icon">
          {expanded ? "\u25BE" : "\u25B8"}
        </span>
        <span>{displayName}</span>
      </button>
      {expanded && <div className="tag-category-body">{children}</div>}
    </div>
  );
}
