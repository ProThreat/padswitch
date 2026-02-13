import type { Profile } from "../types/controller";

interface PresetListProps {
  profiles: Profile[];
  activeProfileId: string | null;
  onActivate: (profileId: string) => Promise<void>;
  onDelete: (profileId: string) => Promise<void>;
}

export default function PresetList({
  profiles,
  activeProfileId,
  onActivate,
  onDelete,
}: PresetListProps) {
  if (profiles.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-icon">&#127918;</div>
        <h3>No presets yet</h3>
        <p>
          Go to the Manual tab, set up your controllers, then save as a preset.
        </p>
      </div>
    );
  }

  return (
    <div className="preset-grid">
      {profiles.map((profile) => {
        const isActive = profile.id === activeProfileId;
        return (
          <div
            key={profile.id}
            className={`preset-card${isActive ? " preset-active" : ""}`}
          >
            <div className="preset-card-body" onClick={() => onActivate(profile.id)}>
              <div className="preset-name">{profile.name}</div>
              <div className="preset-meta">
                <span className="preset-mode">
                  {profile.routing_mode === "Force" ? "Force" : "Minimal"}
                </span>
                <span className="preset-slots">
                  {profile.assignments.length} controller
                  {profile.assignments.length !== 1 ? "s" : ""}
                </span>
              </div>
              {isActive && <span className="preset-badge">Active</span>}
            </div>
            <button
              className="preset-delete"
              onClick={(e) => {
                e.stopPropagation();
                onDelete(profile.id);
              }}
              title="Delete preset"
            >
              &#x2715;
            </button>
          </div>
        );
      })}
    </div>
  );
}
