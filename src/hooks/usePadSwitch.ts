import { useState, useEffect, useCallback } from "react";
import { arrayMove } from "@dnd-kit/sortable";
import type {
  PhysicalDevice,
  DriverStatus,
  Profile,
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
  getProfiles,
  getSettings,
  saveProfile,
  activateProfile,
  deleteProfile,
  updateSettings,
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
  const [forwarding, setForwarding] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [devs, drivers, fwd, loadedProfiles, settings] = await Promise.all([
        getConnectedDevices(),
        checkDriverStatus(),
        isForwarding(),
        getProfiles(),
        getSettings(),
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
      setDevices((prev) => applyAssignmentsToDevices(prev, payload.assignments));
      applyAssignments(payload.assignments).catch(console.error);
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

  const dismissError = useCallback(() => setError(null), []);

  return {
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
  };
}
