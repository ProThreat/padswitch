import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PhysicalDevice, SlotAssignment, RoutingMode } from "../types/controller";

export interface DeviceChangePayload {
  devices: PhysicalDevice[];
}

export interface ForwardingStatusPayload {
  active: boolean;
  error?: string;
}

export interface ProfileActivatedPayload {
  profile_id: string | null;
  assignments: SlotAssignment[];
  routing_mode: RoutingMode;
}

export function onDeviceChange(
  callback: (payload: DeviceChangePayload) => void
): Promise<UnlistenFn> {
  return listen<DeviceChangePayload>("device-change", (event) => {
    callback(event.payload);
  });
}

export function onForwardingStatus(
  callback: (payload: ForwardingStatusPayload) => void
): Promise<UnlistenFn> {
  return listen<ForwardingStatusPayload>("forwarding-status", (event) => {
    callback(event.payload);
  });
}

export function onProfileActivated(
  callback: (payload: ProfileActivatedPayload) => void
): Promise<UnlistenFn> {
  return listen<ProfileActivatedPayload>("profile-activated", (event) => {
    callback(event.payload);
  });
}
