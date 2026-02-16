import { useState, useEffect, useCallback } from "react";
import { arrayMove } from "@dnd-kit/sortable";
import type {
  PhysicalDevice,
  DriverStatus,
  Profile,
  GameRule,
  SlotAssignment,
  RoutingMode,
} from "../types/controller";
import {
  getConnectedDevices,
  checkDriverStatus,
  toggleDevice,
  applyAssignments,
  startForwarding,
  stopForwarding,
  isForwarding,
  isElevated,
  detectXInputSlot,
  confirmDeviceSlot,
  getProfiles,
  getGameRules,
  addGameRule,
  deleteGameRule,
  toggleGameRule,
  startProcessWatcher,
  stopProcessWatcher,
  isWatcherRunning,
  getSettings,
  saveProfile,
  activateProfile,
  deleteProfile,
  updateSettings,
  resetAll,
} from "../lib/ipc";
import {
  onDeviceChange,
  onForwardingStatus,
  onProfileActivated,
} from "../lib/events";

function currentAssignments(devices: PhysicalDevice[]): SlotAssignment[] {
  return devices.map((device, slot) => ({
    device_id: device.id,
    slot,
    enabled: !device.hidden,
  }));
}

function applyAssignmentsToDevices(
  devices: PhysicalDevice[],
  assignments: SlotAssignment[]
): PhysicalDevice[] {
  const byId = new Map(devices.map((device) => [device.id, device]));
  const usedIds = new Set<string>();

  const ordered = assignments
    .slice()
    .sort((a, b) => a.slot - b.slot)
    .map((assignment) => {
      const device = byId.get(assignment.device_id);
      if (!device) return null;
      usedIds.add(device.id);
      return { ...device, hidden: !assignment.enabled };
    })
    .filter((device): device is PhysicalDevice => device !== null);

  const remaining = devices.filter((device) => !usedIds.has(device.id));
  return [...ordered, ...remaining];
}

export function usePadSwitch() {
  const [devices, setDevices] = useState<PhysicalDevice[]>([]);
  const [driverStatus, setDriverStatus] = useState<DriverStatus | null>(null);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [activeProfileId, setActiveProfileId] = useState<string | null>(null);
  const [routingMode, setRoutingMode] = useState<RoutingMode>("Minimal");
  const [gameRules, setGameRules] = useState<GameRule[]>([]);
  const [watcherRunning, setWatcherRunning] = useState(false);
  const [elevated, setElevated] = useState(true); // assume true until checked
  const [identifying, setIdentifying] = useState<string | null>(null); // device ID being identified
  const [forwarding, setForwarding] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [devs, drivers, fwd, loadedProfiles, settings, elev, rules, watching] = await Promise.all([
        getConnectedDevices(),
        checkDriverStatus(),
        isForwarding(),
        getProfiles(),
        getSettings(),
        isElevated(),
        getGameRules(),
        isWatcherRunning(),
      ]);
      const activeProfile = loadedProfiles.find(
        (profile) => profile.id === settings.active_profile_id
      );

      setDevices(
        activeProfile
          ? applyAssignmentsToDevices(devs, activeProfile.assignments)
          : devs
      );
      setDriverStatus(drivers);
      setForwarding(fwd);
      setProfiles(loadedProfiles);
      setActiveProfileId(settings.active_profile_id);
      setRoutingMode(activeProfile?.routing_mode ?? "Minimal");
      setElevated(elev);
      setGameRules(rules);
      setWatcherRunning(watching);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Subscribe to Tauri events
  useEffect(() => {
    const unlistenDevice = onDeviceChange((payload) => {
      setDevices(payload.devices);
    });
    const unlistenForwarding = onForwardingStatus((payload) => {
      setForwarding(payload.active);
      if (payload.error) {
        setError(payload.error);
      }
    });
    const unlistenProfile = onProfileActivated((payload) => {
      setActiveProfileId(payload.profile_id);
      setRoutingMode(payload.routing_mode);
      if (payload.profile_id) {
        setDevices((prev) => applyAssignmentsToDevices(prev, payload.assignments));
        applyAssignments(payload.assignments).catch(console.error);
      }
    });

    return () => {
      unlistenDevice.then((fn) => fn());
      unlistenForwarding.then((fn) => fn());
      unlistenProfile.then((fn) => fn());
    };
  }, []);

  const handleReorder = useCallback((activeId: string, overId: string) => {
    setDevices((prev) => {
      const oldIndex = prev.findIndex((d) => d.id === activeId);
      const newIndex = prev.findIndex((d) => d.id === overId);
      if (oldIndex < 0 || newIndex < 0) {
        return prev;
      }
      const reordered = arrayMove(prev, oldIndex, newIndex);

      const assignments = currentAssignments(reordered);
      applyAssignments(assignments).catch(console.error);

      return reordered;
    });
  }, []);

  const handleToggle = useCallback(async (deviceId: string, hidden: boolean) => {
    try {
      await toggleDevice(deviceId, hidden);
      setDevices((prev) =>
        prev.map((d) => (d.id === deviceId ? { ...d, hidden } : d))
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const handleStartStop = useCallback(async () => {
    try {
      if (forwarding) {
        await stopForwarding();
        setForwarding(false);
      } else {
        const assignments = currentAssignments(devices);
        await applyAssignments(assignments);
        await startForwarding();
        setForwarding(true);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [forwarding, devices]);

  const handleSaveProfile = useCallback(
    async (name: string, mode: RoutingMode) => {
      try {
        const assignments = currentAssignments(devices);
        const profile = await saveProfile(name, assignments, mode);
        const nextProfiles = [...profiles, profile];
        setProfiles(nextProfiles);
        setActiveProfileId(profile.id);
        setRoutingMode(mode);

        const settings = await getSettings();
        await updateSettings({ ...settings, active_profile_id: profile.id });
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [devices, profiles]
  );

  const handleActivateProfile = useCallback(async (profileId: string) => {
    try {
      const assignments = await activateProfile(profileId);
      await applyAssignments(assignments);
      setDevices((prev) => applyAssignmentsToDevices(prev, assignments));
      setActiveProfileId(profileId);
      const profile = profiles.find((p) => p.id === profileId);
      setRoutingMode(profile?.routing_mode ?? "Minimal");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [profiles]);

  const handleDeleteProfile = useCallback(
    async (profileId: string) => {
      try {
        await deleteProfile(profileId);
        const nextProfiles = profiles.filter((profile) => profile.id !== profileId);
        setProfiles(nextProfiles);
        if (activeProfileId === profileId) {
          setActiveProfileId(null);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [activeProfileId, profiles]
  );

  const handleAddGameRule = useCallback(
    async (exeName: string, profileId: string) => {
      try {
        const rule = await addGameRule(exeName, profileId);
        setGameRules((prev) => [...prev, rule]);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    []
  );

  const handleDeleteGameRule = useCallback(async (ruleId: string) => {
    try {
      await deleteGameRule(ruleId);
      setGameRules((prev) => prev.filter((r) => r.id !== ruleId));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const handleToggleGameRule = useCallback(
    async (ruleId: string, enabled: boolean) => {
      try {
        await toggleGameRule(ruleId, enabled);
        setGameRules((prev) =>
          prev.map((r) => (r.id === ruleId ? { ...r, enabled } : r))
        );
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    []
  );

  const handleToggleWatcher = useCallback(async (start: boolean) => {
    try {
      if (start) {
        await startProcessWatcher();
      } else {
        await stopProcessWatcher();
      }
      setWatcherRunning(start);
      // Persist the setting
      const settings = await getSettings();
      await updateSettings({ ...settings, auto_switch: start });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const handleIdentifyDevice = useCallback(async (deviceId: string) => {
    try {
      setIdentifying(deviceId);
      const slot = await detectXInputSlot();
      if (slot !== null) {
        await confirmDeviceSlot(deviceId, slot);
        setDevices((prev) =>
          prev.map((d) =>
            d.id === deviceId ? { ...d, xinput_slot: slot } : d
          )
        );
      }
      return slot;
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      return null;
    } finally {
      setIdentifying(null);
    }
  }, []);

  const handleReset = useCallback(async () => {
    try {
      await resetAll();
      setForwarding(false);
      setActiveProfileId(null);
      setWatcherRunning(false);
      setRoutingMode("Minimal");
      // Refresh device list to show current (reset) state
      const devs = await getConnectedDevices();
      setDevices(devs);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const dismissError = useCallback(() => setError(null), []);

  return {
    devices,
    driverStatus,
    elevated,
    identifying,
    profiles,
    activeProfileId,
    routingMode,
    setRoutingMode,
    gameRules,
    watcherRunning,
    forwarding,
    loading,
    error,
    refresh,
    dismissError,
    handleReorder,
    handleToggle,
    handleStartStop,
    handleIdentifyDevice,
    handleSaveProfile,
    handleActivateProfile,
    handleDeleteProfile,
    handleAddGameRule,
    handleDeleteGameRule,
    handleToggleGameRule,
    handleToggleWatcher,
    handleReset,
  };
}
