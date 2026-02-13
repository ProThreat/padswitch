import { useState } from "react";
import { usePadSwitch } from "./hooks/usePadSwitch";
import ControllerList from "./components/ControllerList";
import DriverStatus from "./components/DriverStatus";
import StatusBar from "./components/StatusBar";
import AboutPanel from "./components/AboutPanel";
import PresetList from "./components/PresetList";
import type { RoutingMode } from "./types/controller";
import "./App.css";

type Tab = "presets" | "manual";

function App() {
  const {
    devices,
    driverStatus,
    profiles,
    activeProfileId,
    routingMode,
    setRoutingMode,
    forwarding,
    loading,
    error,
    refresh,
    dismissError,
    handleReorder,
    handleToggle,
    handleStartStop,
    handleSaveProfile,
    handleActivateProfile,
    handleDeleteProfile,
  } = usePadSwitch();

  const [tab, setTab] = useState<Tab>("presets");
  const [aboutOpen, setAboutOpen] = useState(false);
  const [savePresetName, setSavePresetName] = useState("");

  if (loading) {
    return (
      <div className="app">
        <div className="loading-state">Loading...</div>
      </div>
    );
  }

  async function handleSaveAsPreset() {
    const trimmed = savePresetName.trim();
    if (!trimmed) return;
    await handleSaveProfile(trimmed, routingMode);
    setSavePresetName("");
  }

  return (
    <div className="app">
      <header className="app-header">
        <h1>PadSwitch</h1>
        <button
          className="btn btn-ghost"
          onClick={() => setAboutOpen(true)}
          title="About"
        >
          ?
        </button>
      </header>

      {error && (
        <div className="error-banner">
          <span>{error}</span>
          <button className="btn btn-ghost" onClick={dismissError}>
            x
          </button>
        </div>
      )}

      <DriverStatus status={driverStatus} />

      <nav className="tab-bar">
        <button
          className={`tab-btn${tab === "presets" ? " tab-active" : ""}`}
          onClick={() => setTab("presets")}
        >
          Presets
        </button>
        <button
          className={`tab-btn${tab === "manual" ? " tab-active" : ""}`}
          onClick={() => setTab("manual")}
        >
          Manual
        </button>
      </nav>

      <div className="app-body">
        {tab === "presets" && (
          <PresetList
            profiles={profiles}
            activeProfileId={activeProfileId}
            onActivate={handleActivateProfile}
            onDelete={handleDeleteProfile}
          />
        )}

        {tab === "manual" && (
          <>
            <div className="section-header">
              <h2>Controllers</h2>
              <p className="section-hint">Drag to set P1-P4 slot order</p>
            </div>

            <ControllerList
              devices={devices}
              onReorder={handleReorder}
              onToggle={handleToggle}
            />

            <section className="save-preset-section">
              <div className="section-header">
                <h2>Save as preset</h2>
              </div>
              <div className="save-preset-row">
                <input
                  type="text"
                  value={savePresetName}
                  onChange={(e) => setSavePresetName(e.target.value)}
                  placeholder="e.g. Wooting + Controller"
                  maxLength={64}
                />
                <select
                  value={routingMode}
                  onChange={(e) =>
                    setRoutingMode(e.target.value as RoutingMode)
                  }
                >
                  <option value="Minimal">Minimal</option>
                  <option value="Force">Force</option>
                </select>
                <button
                  className="btn btn-primary"
                  onClick={handleSaveAsPreset}
                  disabled={savePresetName.trim().length === 0}
                >
                  Save
                </button>
              </div>
              {routingMode === "Force" && (
                <p className="mode-warning">
                  Requires HidHide + ViGEmBus. May conflict with Steam Input.
                </p>
              )}
            </section>
          </>
        )}
      </div>

      <StatusBar
        forwarding={forwarding}
        deviceCount={devices.length}
        onStartStop={handleStartStop}
        onRefresh={refresh}
      />

      <AboutPanel open={aboutOpen} onClose={() => setAboutOpen(false)} />
    </div>
  );
}

export default App;
