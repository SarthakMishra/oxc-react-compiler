// XS tier - Inspired by cal.com booking status badges
import { useMemo } from 'react';

type BookingStatus = 'confirmed' | 'pending' | 'cancelled' | 'completed';

interface StatusBadgeProps {
  status: BookingStatus;
}

export function StatusBadge({ status }: StatusBadgeProps) {
  const config = useMemo(() => {
    switch (status) {
      case 'confirmed': return { label: 'Confirmed', color: 'bg-green-100 text-green-800' };
      case 'pending': return { label: 'Pending', color: 'bg-yellow-100 text-yellow-800' };
      case 'cancelled': return { label: 'Cancelled', color: 'bg-red-100 text-red-800' };
      case 'completed': return { label: 'Completed', color: 'bg-gray-100 text-gray-800' };
    }
  }, [status]);

  return <span className={`px-2 py-1 rounded text-sm ${config.color}`}>{config.label}</span>;
}
