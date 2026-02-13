interface AboutPanelProps {
  open: boolean;
  onClose: () => void;
}

export default function AboutPanel({ open, onClose }: AboutPanelProps) {
  if (!open) return null;

  return (
    <div className="about-overlay" onClick={onClose}>
      <div className="about-panel" onClick={(e) => e.stopPropagation()}>
        <h2>PadSwitch</h2>
        <p className="about-version">v0.1.0</p>
        <p>
          Controller manager for toggling, reordering, and forwarding XInput
          devices.
        </p>

        <h3>How it works</h3>
        <p>
          PadSwitch uses HidHide to hide your real controllers from games, then
          creates virtual controllers via ViGEmBus in your preferred slot order.
          Input is forwarded from hidden devices to virtual ones at 1000Hz.
        </p>

        <h3>Credits & Attribution</h3>
        <ul className="about-credits">
          <li>
            <a
              href="https://github.com/nefarius/HidHide"
              target="_blank"
              rel="noreferrer"
            >
              <strong>HidHide</strong>
            </a>{" "}
            by Nefarius (Benjamin Höglinger-Stelzer) — Device hiding filter
            driver
          </li>
          <li>
            <a
              href="https://github.com/nefarius/ViGEmBus"
              target="_blank"
              rel="noreferrer"
            >
              <strong>ViGEmBus</strong>
            </a>{" "}
            by Nefarius — Virtual gamepad bus driver
          </li>
          <li>
            <a
              href="https://github.com/CasualX/vigem-client"
              target="_blank"
              rel="noreferrer"
            >
              <strong>vigem-client</strong>
            </a>{" "}
            crate by CasualX — Rust bindings for ViGEmBus
          </li>
          <li>
            <a
              href="https://docs.rs/rusty-xinput"
              target="_blank"
              rel="noreferrer"
            >
              <strong>rusty-xinput</strong>
            </a>{" "}
            — XInput Rust bindings
          </li>
        </ul>

        <button className="btn btn-secondary about-close" onClick={onClose}>
          Close
        </button>
      </div>
    </div>
  );
}
