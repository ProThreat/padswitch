import type { DriverStatus as DriverStatusType } from "../types/controller";

interface DriverStatusProps {
  status: DriverStatusType | null;
}

export default function DriverStatus({ status }: DriverStatusProps) {
  if (!status) return null;

  const allInstalled = status.hidhide_installed && status.vigembus_installed;
  if (allInstalled) return null;

  return (
    <div className="driver-banner">
      <div className="driver-banner-icon">⚠️</div>
      <div className="driver-banner-content">
        <strong>Required drivers missing</strong>
        <p>PadSwitch needs these drivers to hide and remap controllers:</p>
        <ul>
          {!status.hidhide_installed && (
            <li>
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
          )}
          {!status.vigembus_installed && (
            <li>
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
          )}
        </ul>
        <p className="driver-note">
          Install the drivers, then restart PadSwitch.
        </p>
      </div>
    </div>
  );
}
