import type { ReactNode } from "react";
import type {
  DriverStatus as DriverStatusType,
  RoutingMode,
} from "../types/controller";

interface DriverStatusProps {
  status: DriverStatusType | null;
  routingMode: RoutingMode;
  elevated: boolean;
}

export default function DriverStatus({
  status,
  routingMode,
  elevated,
}: DriverStatusProps) {
  const warnings: ReactNode[] = [];

  // Minimal mode: needs admin elevation for SetupDi disable/enable
  if (routingMode === "Minimal" && !elevated) {
    warnings.push(
      <div key="elevation" className="driver-banner">
        <div className="driver-banner-icon">!</div>
        <div className="driver-banner-content">
          <strong>Administrator required</strong>
          <p>
            Minimal mode uses SetupDi to disable/re-enable controllers, which
            requires admin privileges. Restart PadSwitch as Administrator.
          </p>
        </div>
      </div>
    );
  }

  // Force mode: needs HidHide + ViGEmBus installed
  if (routingMode === "Force" && status) {
    const missing: ReactNode[] = [];

    if (!status.hidhide_installed) {
      missing.push(
        <li key="hidhide">
          <strong>HidHide</strong> — Hides physical devices from games
          <br />
          <a
            href="https://github.com/nefarius/HidHide/releases"
            target="_blank"
            rel="noreferrer"
          >
            Download from GitHub
          </a>
        </li>
      );
    }

    if (!status.vigembus_installed) {
      missing.push(
        <li key="vigem">
          <strong>ViGEmBus</strong> — Creates virtual controllers
          <br />
          <a
            href="https://github.com/nefarius/ViGEmBus/releases"
            target="_blank"
            rel="noreferrer"
          >
            Download from GitHub
          </a>
        </li>
      );
    }

    if (missing.length > 0) {
      warnings.push(
        <div key="drivers" className="driver-banner">
          <div className="driver-banner-icon">!</div>
          <div className="driver-banner-content">
            <strong>Required drivers missing for Force mode</strong>
            <p>
              Force mode needs these drivers to hide and remap controllers:
            </p>
            <ul>{missing}</ul>
            <p className="driver-note">
              Install the drivers, then restart PadSwitch.
            </p>
          </div>
        </div>
      );
    }
  }

  if (warnings.length === 0) return null;

  return <>{warnings}</>;
}
