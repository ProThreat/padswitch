import { useState, useEffect, useCallback } from "react";
import { arrayMove } from "@dnd-kit/sortable";
import type {
  PhysicalDevice,
  DriverStatus as DriverStatusType,
} from "./types/controller";
import {
  getConnectedDevices,
  checkDriverStatus,
  toggleDevice,
  applyAssignments,
  startForwarding,
  stopForwarding,
  isForwarding,
} from "./lib/ipc";
import ControllerList from "./components/ControllerList";
import DriverStatus from "./components/DriverStatus";
import StatusBar from "./components/StatusBar";
import AboutPanel from "./components/AboutPanel";
import "./App.css";

function App() {
  const [devices, setDevices] = useState<PhysicalDevice[]>([]);
  const [driverStatus, setDriverStatus] = useState<DriverStatusType | null>(
    null
  );
  const [forwarding, setForwarding] = useState(false);
  const [aboutOpen, setAboutOpen] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const [devs, drivers, fwd] = await Promise.all([
        getConnectedDevices(),
        checkDriverStatus(),
        isForwarding(),
      ]);
      setDevices(devs);
      setDriverStatus(drivers);
      setForwarding(fwd);
    } catch (err) {
      console.error("Failed to refresh:", err);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleReorder = useCallback(
    (activeId: string, overId: string) => {
      setDevices((prev) => {
        const oldIndex = prev.findIndex((d) => d.id === activeId);
        const newIndex = prev.findIndex((d) => d.id === overId);
        const reordered = arrayMove(prev, oldIndex, newIndex);

        // Send updated assignments to backend
        const assignments = reordered.map((d, i) => ({
          device_id: d.id,
          slot: i,
          enabled: !d.hidden,
        }));
        applyAssignments(assignments).catch(console.error);

        return reordered;
      });
    },
    []
  );

  const handleToggle = useCallback(async (deviceId: string, hidden: boolean) => {
    try {
      await toggleDevice(deviceId, hidden);
      setDevices((prev) =>
        prev.map((d) => (d.id === deviceId ? { ...d, hidden } : d))
      );
    } catch (err) {
      console.error("Failed to toggle device:", err);
    }
  }, []);

  const handleStartStop = useCallback(async () => {
    try {
      if (forwarding) {
        await stopForwarding();
        setForwarding(false);
      } else {
        // Apply current order as assignments before starting
        const assignments = devices.map((d, i) => ({
          device_id: d.id,
          slot: i,
          enabled: !d.hidden,
        }));
        await applyAssignments(assignments);
        await startForwarding();
        setForwarding(true);
      }
    } catch (err) {
      console.error("Forwarding error:", err);
    }
  }, [forwarding, devices]);

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

      <DriverStatus status={driverStatus} />

      <div className="app-body">
        <div className="section-header">
          <h2>Controllers</h2>
          <p className="section-hint">Drag to reorder XInput slots</p>
        </div>

        <ControllerList
          devices={devices}
          onReorder={handleReorder}
          onToggle={handleToggle}
        />
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
