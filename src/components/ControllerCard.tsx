import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { PhysicalDevice } from "../types/controller";

interface ControllerCardProps {
  device: PhysicalDevice;
  slot: number;
  identifying: boolean;
  onToggle: (deviceId: string, hidden: boolean) => void;
  onIdentify: (deviceId: string) => void;
}

function deviceIcon(type: string): string {
  switch (type) {
    case "XInput":
      return "\uD83C\uDFAE";
    case "DirectInput":
      return "\uD83D\uDD79\uFE0F";
    default:
      return "\uD83D\uDD0C";
  }
}

export default function ControllerCard({
  device,
  slot,
  identifying,
  onToggle,
  onIdentify,
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
          {device.xinput_slot !== null && (
            <span className="card-type">Slot {device.xinput_slot}</span>
          )}
          {!device.connected && (
            <span className="card-status disconnected">Disconnected</span>
          )}
          {device.hidden && (
            <span className="card-status hidden">Hidden</span>
          )}
        </div>
      </div>

      {device.device_type === "XInput" && (
        <button
          className="btn-identify"
          onClick={() => onIdentify(device.id)}
          disabled={identifying}
          title="Press a button on this controller to identify its XInput slot"
        >
          {identifying ? "..." : "ID"}
        </button>
      )}

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
