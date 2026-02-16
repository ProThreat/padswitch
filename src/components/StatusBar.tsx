interface StatusBarProps {
  forwarding: boolean;
  deviceCount: number;
  onStartStop: () => void;
  onRefresh: () => void;
  onReset: () => void;
}

export default function StatusBar({
  forwarding,
  deviceCount,
  onStartStop,
  onRefresh,
  onReset,
}: StatusBarProps) {
  return (
    <div className="status-bar">
      <div className="status-indicator">
        <span
          className={`status-dot ${forwarding ? "active" : "inactive"}`}
        />
        <span className="status-text">
          {forwarding ? "Forwarding active" : "Forwarding stopped"}
        </span>
      </div>

      <div className="status-info">
        {deviceCount} controller{deviceCount !== 1 ? "s" : ""} detected
      </div>

      <div className="status-actions">
        <button
          className="btn btn-reset"
          onClick={onReset}
          title="Reset all controllers to default state"
        >
          Reset
        </button>
        <button className="btn btn-secondary" onClick={onRefresh}>
          Refresh
        </button>
        <button
          className={`btn ${forwarding ? "btn-danger" : "btn-primary"}`}
          onClick={onStartStop}
          disabled={deviceCount === 0}
        >
          {forwarding ? "Stop" : "Start"}
        </button>
      </div>
    </div>
  );
}
