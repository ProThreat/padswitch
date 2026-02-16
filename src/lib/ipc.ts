import { invoke } from "@tauri-apps/api/core";
import type {
  PhysicalDevice,
  DriverStatus,
  SlotAssignment,
  Profile,
  GameRule,
  Settings,
  RoutingMode,
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

export const saveProfile = (name: string, assignments: SlotAssignment[], routingMode: RoutingMode) =>
  invoke<Profile>("save_profile", { name, assignments, routingMode });

export const deleteProfile = (profileId: string) =>
  invoke<void>("delete_profile", { profileId });

export const activateProfile = (profileId: string) =>
  invoke<SlotAssignment[]>("activate_profile", { profileId });

// Environment
export const isElevated = () => invoke<boolean>("is_elevated");

// Device identification — polls XInput for button press, returns slot 0-3 or null
export const detectXInputSlot = () => invoke<number | null>("detect_xinput_slot");

// Confirm a device's XInput slot after identification
export const confirmDeviceSlot = (deviceId: string, xinputSlot: number) =>
  invoke<void>("confirm_device_slot", { deviceId, xinputSlot });

// Game rules
export const getGameRules = () => invoke<GameRule[]>("get_game_rules");

export const addGameRule = (exeName: string, profileId: string) =>
  invoke<GameRule>("add_game_rule", { exeName, profileId });

export const deleteGameRule = (ruleId: string) =>
  invoke<void>("delete_game_rule", { ruleId });

export const toggleGameRule = (ruleId: string, enabled: boolean) =>
  invoke<void>("toggle_game_rule", { ruleId, enabled });

// Process watcher
export const startProcessWatcher = () => invoke<void>("start_process_watcher");

export const stopProcessWatcher = () => invoke<void>("stop_process_watcher");

export const isWatcherRunning = () => invoke<boolean>("is_watcher_running");

// Reset
export const resetAll = () => invoke<void>("reset_all");

// Settings
export const getSettings = () => invoke<Settings>("get_settings");

export const updateSettings = (settings: Settings) =>
  invoke<void>("update_settings", { settings });
