// S tier - Inspired by cal.com time slot picker
import { useState, useMemo, useCallback } from 'react';

interface TimeSlot {
  time: string;
  available: boolean;
}

interface TimeSlotPickerProps {
  slots: TimeSlot[];
  selectedDate: string;
  onSelect: (time: string) => void;
  timezone?: string;
}

export function TimeSlotPicker({ slots, selectedDate, onSelect, timezone = 'UTC' }: TimeSlotPickerProps) {
  const [selectedSlot, setSelectedSlot] = useState<string | null>(null);

  const availableSlots = useMemo(
    () => slots.filter((s) => s.available),
    [slots]
  );

  const morningSlots = useMemo(
    () => availableSlots.filter((s) => parseInt(s.time) < 12),
    [availableSlots]
  );

  const afternoonSlots = useMemo(
    () => availableSlots.filter((s) => parseInt(s.time) >= 12),
    [availableSlots]
  );

  const handleSelect = useCallback(
    (time: string) => {
      setSelectedSlot(time);
      onSelect(time);
    },
    [onSelect]
  );

  return (
    <div>
      <h3>{selectedDate} ({timezone})</h3>
      {availableSlots.length === 0 ? (
        <p>No available slots</p>
      ) : (
        <>
          {morningSlots.length > 0 && (
            <div>
              <h4>Morning</h4>
              {morningSlots.map((slot) => (
                <button
                  key={slot.time}
                  onClick={() => handleSelect(slot.time)}
                  className={selectedSlot === slot.time ? 'selected' : ''}
                >
                  {slot.time}
                </button>
              ))}
            </div>
          )}
          {afternoonSlots.length > 0 && (
            <div>
              <h4>Afternoon</h4>
              {afternoonSlots.map((slot) => (
                <button
                  key={slot.time}
                  onClick={() => handleSelect(slot.time)}
                  className={selectedSlot === slot.time ? 'selected' : ''}
                >
                  {slot.time}
                </button>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}
