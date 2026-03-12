// M tier - Inspired by cal.com BookingListItem with scheduling logic
import { useState, useMemo, useCallback, useEffect } from 'react';

interface Booking {
  id: string;
  title: string;
  startTime: string;
  endTime: string;
  attendees: { name: string; email: string; status: 'accepted' | 'pending' | 'declined' }[];
  status: 'confirmed' | 'pending' | 'cancelled';
  location?: string;
  notes?: string;
}

interface BookingListProps {
  bookings: Booking[];
  onCancel: (id: string) => void;
  onReschedule: (id: string) => void;
  onConfirm: (id: string) => void;
  filter?: 'all' | 'upcoming' | 'past';
}

export function BookingList({ bookings, onCancel, onReschedule, onConfirm, filter = 'all' }: BookingListProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [sortBy, setSortBy] = useState<'date' | 'title'>('date');

  const now = useMemo(() => new Date().toISOString(), []);

  const filteredBookings = useMemo(() => {
    let result = bookings;

    if (filter === 'upcoming') {
      result = result.filter((b) => b.startTime > now && b.status !== 'cancelled');
    } else if (filter === 'past') {
      result = result.filter((b) => b.endTime < now);
    }

    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        (b) =>
          b.title.toLowerCase().includes(q) ||
          b.attendees.some((a) => a.name.toLowerCase().includes(q) || a.email.toLowerCase().includes(q))
      );
    }

    return result;
  }, [bookings, filter, now, searchQuery]);

  const sortedBookings = useMemo(() => {
    return [...filteredBookings].sort((a, b) => {
      if (sortBy === 'date') return a.startTime.localeCompare(b.startTime);
      return a.title.localeCompare(b.title);
    });
  }, [filteredBookings, sortBy]);

  const stats = useMemo(() => ({
    total: sortedBookings.length,
    confirmed: sortedBookings.filter((b) => b.status === 'confirmed').length,
    pending: sortedBookings.filter((b) => b.status === 'pending').length,
    cancelled: sortedBookings.filter((b) => b.status === 'cancelled').length,
  }), [sortedBookings]);

  const toggleExpanded = useCallback((id: string) => {
    setExpandedId((prev) => (prev === id ? null : id));
  }, []);

  const handleCancel = useCallback(
    (id: string) => {
      onCancel(id);
      setExpandedId(null);
    },
    [onCancel]
  );

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <div className="flex gap-2 text-sm">
          <span>{stats.total} total</span>
          <span className="text-green-600">{stats.confirmed} confirmed</span>
          <span className="text-yellow-600">{stats.pending} pending</span>
          <span className="text-red-600">{stats.cancelled} cancelled</span>
        </div>
        <div className="flex gap-2">
          <input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search bookings..."
            className="border rounded px-2 py-1"
          />
          <select value={sortBy} onChange={(e) => setSortBy(e.target.value as 'date' | 'title')}>
            <option value="date">Sort by date</option>
            <option value="title">Sort by title</option>
          </select>
        </div>
      </div>

      {sortedBookings.length === 0 ? (
        <p className="text-gray-500 text-center py-8">No bookings found</p>
      ) : (
        <ul className="divide-y">
          {sortedBookings.map((booking) => (
            <li key={booking.id} className="py-3">
              <div className="flex justify-between items-start cursor-pointer" onClick={() => toggleExpanded(booking.id)}>
                <div>
                  <h3 className="font-medium">{booking.title}</h3>
                  <p className="text-sm text-gray-500">
                    {booking.startTime} - {booking.endTime}
                  </p>
                  <p className="text-sm">{booking.attendees.length} attendee(s)</p>
                </div>
                <span className={`px-2 py-1 rounded text-xs ${
                  booking.status === 'confirmed' ? 'bg-green-100' :
                  booking.status === 'pending' ? 'bg-yellow-100' : 'bg-red-100'
                }`}>
                  {booking.status}
                </span>
              </div>

              {expandedId === booking.id && (
                <div className="mt-2 pl-4 border-l-2">
                  {booking.location && <p className="text-sm">📍 {booking.location}</p>}
                  {booking.notes && <p className="text-sm text-gray-600">{booking.notes}</p>}
                  <div className="mt-2">
                    <h4 className="text-xs font-semibold uppercase">Attendees</h4>
                    {booking.attendees.map((a, i) => (
                      <div key={i} className="text-sm flex justify-between">
                        <span>{a.name} ({a.email})</span>
                        <span>{a.status}</span>
                      </div>
                    ))}
                  </div>
                  <div className="mt-2 flex gap-2">
                    {booking.status === 'pending' && (
                      <button onClick={() => onConfirm(booking.id)} className="text-green-600">Confirm</button>
                    )}
                    <button onClick={() => onReschedule(booking.id)} className="text-blue-600">Reschedule</button>
                    {booking.status !== 'cancelled' && (
                      <button onClick={() => handleCancel(booking.id)} className="text-red-600">Cancel</button>
                    )}
                  </div>
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
