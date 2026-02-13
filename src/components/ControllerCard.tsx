import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { PhysicalDevice } from "../types/controller";

interface ControllerCardProps {
  device: PhysicalDevice;
  slot: number;
  onToggle: (deviceId: string, hidden: boolean) => void;
}

function deviceIcon(type: string): string {
  switch (type) {
    case "XInput":
      return "🎮";
    case "DirectInput":
      return "🕹️";
    default:
      return "🔌";
  }
}

export default function ControllerCard({
  device,
  slot,
  onToggle,
}: ControllerCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id: device.id });

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`controller-card ${device.hidden ? "hidden-device" : ""} ${!device.connected ? "disconnected" : ""}`}
    >
      <div className="card-drag-handle" {...attributes} {...listeners}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <circle cx="5" cy="3" r="1.5" />
          <circle cx="11" cy="3" r="1.5" />
          <circle cx="5" cy="8" r="1.5" />
          <circle cx="11" cy="8" r="1.5" />
          <circle cx="5" cy="13" r="1.5" />
          <circle cx="11" cy="13" r="1.5" />
        </svg>
      </div>

      <div className="card-slot-badge">
        <span>P{slot + 1}</span>
      </div>

      <div className="card-icon">{deviceIcon(device.device_type)}</div>

      <div className="card-info">
        <div className="card-name">{device.name}</div>
        <div className="card-meta">
          <span className="card-type">{device.device_type}</span>
          {!device.connected && (
            <span className="card-status disconnected">Disconnected</span>
          )}
          {device.hidden && (
            <span className="card-status hidden">Hidden</span>
          )}
        </div>
      </div>

      <label className="card-toggle">
        <input
          type="checkbox"
          checked={!device.hidden}
          onChange={() => onToggle(device.id, !device.hidden)}
        />
        <span className="toggle-slider" />
      </label>
    </div>
  );
}
