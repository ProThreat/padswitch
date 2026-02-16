export type DeviceType = "XInput" | "DirectInput" | "Unknown";

export interface PhysicalDevice {
  id: string;
  name: string;
  instance_path: string;
  device_type: DeviceType;
  hidden: boolean;
  connected: boolean;
  vendor_id: number;
  product_id: number;
  xinput_slot: number | null;
}

export interface SlotAssignment {
  device_id: string;
  slot: number;
  enabled: boolean;
}

export interface DriverStatus {
  hidhide_installed: boolean;
  vigembus_installed: boolean;
  hidhide_version: string | null;
  vigembus_version: string | null;
}

export type RoutingMode = "Minimal" | "Force";

export interface Profile {
  id: string;
  name: string;
  assignments: SlotAssignment[];
  routing_mode: RoutingMode;
}

export interface GameRule {
  id: string;
  exe_name: string;
  profile_id: string;
  enabled: boolean;
}

export interface Settings {
  auto_start: boolean;
  start_minimized: boolean;
  auto_forward_on_launch: boolean;
  auto_switch: boolean;
  active_profile_id: string | null;
}
