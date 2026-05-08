// Phase: 6

interface TagChipProps {
  id: string;
  name: string;
  active: boolean;
  instructionBody: string;
  isCustom: boolean;
  onToggle: (id: string) => void;
  onEdit?: (id: string) => void;
}

export function TagChip({
  id,
  name,
  active,
  instructionBody,
  isCustom,
  onToggle,
  onEdit,
}: TagChipProps) {
  const tooltip =
    instructionBody.length > 100
      ? instructionBody.slice(0, 100) + "..."
      : instructionBody;

  return (
    <button
      className={`tag-chip ${active ? "active" : ""}`}
      title={tooltip}
      onClick={() => onToggle(id)}
      data-testid={`tag-chip-${id}`}
    >
      <span className="tag-chip-name">{name}</span>
      {isCustom && onEdit && (
        <span
          className="tag-edit-icon"
          role="button"
          aria-label={`Edit ${name}`}
          onClick={(e) => {
            e.stopPropagation();
            onEdit(id);
          }}
        >
          &#9998;
        </span>
      )}
    </button>
  );
}
