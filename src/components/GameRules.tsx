import { useState } from "react";
import type { GameRule, Profile } from "../types/controller";

interface GameRulesProps {
  rules: GameRule[];
  profiles: Profile[];
  watcherRunning: boolean;
  onAddRule: (exeName: string, profileId: string) => void;
  onDeleteRule: (ruleId: string) => void;
  onToggleRule: (ruleId: string, enabled: boolean) => void;
  onToggleWatcher: (running: boolean) => void;
}

export default function GameRules({
  rules,
  profiles,
  watcherRunning,
  onAddRule,
  onDeleteRule,
  onToggleRule,
  onToggleWatcher,
}: GameRulesProps) {
  const [exeName, setExeName] = useState("");
  const [profileId, setProfileId] = useState("");

  function handleAdd() {
    const trimmed = exeName.trim();
    if (!trimmed || !profileId) return;
    onAddRule(trimmed, profileId);
    setExeName("");
  }

  function profileName(id: string): string {
    return profiles.find((p) => p.id === id)?.name ?? "(deleted)";
  }

  return (
    <div className="game-rules">
      <div className="section-header">
        <h2>Auto-Switch</h2>
        <label className="card-toggle watcher-toggle">
          <input
            type="checkbox"
            checked={watcherRunning}
            onChange={() => onToggleWatcher(!watcherRunning)}
          />
          <span className="toggle-slider" />
        </label>
      </div>

      <p className="section-hint" style={{ marginBottom: 12 }}>
        {watcherRunning
          ? "Watching for game launches..."
          : "Enable to auto-switch presets when games launch."}
      </p>

      {rules.length > 0 && (
        <div className="rule-list">
          {rules.map((rule) => (
            <div
              key={rule.id}
              className={`rule-card ${!rule.enabled ? "rule-disabled" : ""}`}
            >
              <div className="rule-info">
                <div className="rule-exe">{rule.exe_name}</div>
                <div className="rule-profile">{profileName(rule.profile_id)}</div>
              </div>
              <label className="card-toggle">
                <input
                  type="checkbox"
                  checked={rule.enabled}
                  onChange={() => onToggleRule(rule.id, !rule.enabled)}
                />
                <span className="toggle-slider" />
              </label>
              <button
                className="rule-delete"
                onClick={() => onDeleteRule(rule.id)}
                title="Delete rule"
              >
                x
              </button>
            </div>
          ))}
        </div>
      )}

      {rules.length === 0 && (
        <div className="empty-state" style={{ padding: "24px 20px" }}>
          <p>No game rules yet. Add one below.</p>
        </div>
      )}

      <div className="add-rule-section">
        <div className="section-header">
          <h2>Add Rule</h2>
        </div>
        <div className="add-rule-row">
          <input
            type="text"
            value={exeName}
            onChange={(e) => setExeName(e.target.value)}
            placeholder="e.g. RocketLeague.exe"
            maxLength={128}
          />
          <select
            value={profileId}
            onChange={(e) => setProfileId(e.target.value)}
          >
            <option value="">Select preset...</option>
            {profiles.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
          <button
            className="btn btn-primary"
            onClick={handleAdd}
            disabled={!exeName.trim() || !profileId}
          >
            Add
          </button>
        </div>
      </div>
    </div>
  );
}
