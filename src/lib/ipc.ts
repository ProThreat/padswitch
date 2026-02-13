import { invoke } from "@tauri-apps/api/core";
import type {
  PhysicalDevice,
  DriverStatus,
  SlotAssignment,
  Profile,
  Settings,
} from "../types/controller";

// Device discovery
export const getConnectedDevices = () =>
  invoke<PhysicalDevice[]>("get_connected_devices");

export const checkDriverStatus = () =>
  invoke<DriverStatus>("check_driver_status");

// Device toggling
export const toggleDevice = (deviceId: string, hidden: boolean) =>
  invoke<void>("toggle_device", { deviceId, hidden });

// Forwarding
export const applyAssignments = (assignments: SlotAssignment[]) =>
  invoke<void>("apply_assignments", { assignments });

export const startForwarding = () => invoke<void>("start_forwarding");

export const stopForwarding = () => invoke<void>("stop_forwarding");

export const isForwarding = () => invoke<boolean>("is_forwarding");

// Profiles
export const getProfiles = () => invoke<Profile[]>("get_profiles");

export const saveProfile = (name: string, assignments: SlotAssignment[]) =>
  invoke<Profile>("save_profile", { name, assignments });

export const deleteProfile = (profileId: string) =>
  invoke<void>("delete_profile", { profileId });

export const activateProfile = (profileId: string) =>
  invoke<SlotAssignment[]>("activate_profile", { profileId });

// Settings
export const getSettings = () => invoke<Settings>("get_settings");

export const updateSettings = (settings: Settings) =>
  invoke<void>("update_settings", { settings });
