import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import {
  restrictToVerticalAxis,
  restrictToParentElement,
} from "@dnd-kit/modifiers";
import type { PhysicalDevice } from "../types/controller";
import ControllerCard from "./ControllerCard";

interface ControllerListProps {
  devices: PhysicalDevice[];
  identifying: string | null;
  onReorder: (activeId: string, overId: string) => void;
  onToggle: (deviceId: string, hidden: boolean) => void;
  onIdentify: (deviceId: string) => void;
}

export default function ControllerList({
  devices,
  identifying,
  onReorder,
  onToggle,
  onIdentify,
}: ControllerListProps) {
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 5 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (over && active.id !== over.id) {
      onReorder(String(active.id), String(over.id));
    }
  }

  if (devices.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-icon">🎮</div>
        <h3>No controllers detected</h3>
        <p>Connect a controller and click Refresh</p>
      </div>
    );
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      modifiers={[restrictToVerticalAxis, restrictToParentElement]}
      onDragEnd={handleDragEnd}
    >
      <SortableContext
        items={devices.map((d) => d.id)}
        strategy={verticalListSortingStrategy}
      >
        <div className="controller-list">
          {devices.map((device, index) => (
            <ControllerCard
              key={device.id}
              device={device}
              slot={index}
              identifying={identifying === device.id}
              onToggle={onToggle}
              onIdentify={onIdentify}
            />
          ))}
        </div>
      </SortableContext>
    </DndContext>
  );
}
