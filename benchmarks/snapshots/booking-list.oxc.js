import { c as _c } from "react/compiler-runtime";
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

export function BookingList(t0) {
  const $ = _c(23);
  const { bookings, onCancel, onReschedule, onConfirm, filter } = t0;
  let filteredBookings;
  if ($[0] !== useMemo) {
    const t221 = () => {
      return new Date().toISOString();
    };
    const now = useMemo(t221, []);
    $[0] = useMemo;
  }
  let sortedBookings;
  if ($[1] !== bookings || $[2] !== filter || $[3] !== now || $[4] !== searchQuery || $[5] !== useMemo) {
    const t227 = () => {
      let result = bookings;
      if (filter === "upcoming") {
        const t11 = (b) => {
          t1 = b.startTime > now;
          t1 = b.status !== "cancelled";
          return t1;
        };
        result = result.filter(t11);
      } else {
        if (filter === "past") {
          const t21 = (b) => {
            return b.endTime < now;
          };
          result = result.filter(t21);
        }
      }
      if (searchQuery) {
        const q = searchQuery.toLowerCase();
        const t35 = (b) => {
          t1 = b.title.toLowerCase().includes(q);
          const t14 = (a) => {
            t1 = a.name.toLowerCase().includes(q);
            t1 = a.email.toLowerCase().includes(q);
            return t1;
          };
          t1 = b.attendees.some(t14);
          return t1;
        };
        result = result.filter(t35);
      }
      return result;
    };
    filteredBookings = useMemo(t227, [bookings, filter, now, searchQuery]);
    $[1] = bookings;
    $[2] = filter;
    $[3] = now;
    $[4] = searchQuery;
    $[5] = useMemo;
  }
  let stats;
  if ($[6] !== filteredBookings || $[7] !== sortBy || $[8] !== useMemo) {
    const t237 = () => {
      const t3 = (a, b) => {
        if (sortBy === "date") {
          return a.startTime.localeCompare(b.startTime);
        }
        return a.title.localeCompare(b.title);
      };
      return [...filteredBookings].sort(t3);
    };
    sortedBookings = useMemo(t237, [filteredBookings, sortBy]);
    $[6] = filteredBookings;
    $[7] = sortBy;
    $[8] = useMemo;
  }
  let toggleExpanded;
  if ($[9] !== sortedBookings || $[10] !== useMemo) {
    const t245 = () => {
      const t5 = (b) => {
        return b.status === "confirmed";
      };
      const t10 = (b) => {
        return b.status === "pending";
      };
      const t15 = (b) => {
        return b.status === "cancelled";
      };
      return { total: sortedBookings.length, confirmed: sortedBookings.filter(t5).length, pending: sortedBookings.filter(t10).length, cancelled: sortedBookings.filter(t15).length };
    };
    stats = useMemo(t245, [sortedBookings]);
    $[9] = sortedBookings;
    $[10] = useMemo;
  }
  let handleCancel;
  if ($[11] !== useCallback) {
    const t252 = (id) => {
      const t3 = (prev) => {
        if (prev === id) {
          t6 = null;
        } else {
          t6 = id;
        }
        return t6;
      };
      const t4 = setExpandedId(t3);
      return undefined;
    };
    toggleExpanded = useCallback(t252, []);
    $[11] = useCallback;
  }
  const t258 = (id) => {
    const t5 = onCancel(id);
    const t9 = setExpandedId(null);
    return undefined;
  };
  handleCancel = useCallback(t258, [onCancel]);
  const t297 = (e) => {
    return setSearchQuery(e.target.value);
  };
  const t303 = (e) => {
    return setSortBy(e.target.value);
  };
  if (sortedBookings.length === 0) {
    let t339;
    if ($[12] !== onCancel || $[13] !== searchQuery || $[14] !== sortBy || $[15] !== sortedBookings || $[16] !== sortedBookings || $[17] !== stats || $[18] !== stats || $[19] !== stats || $[20] !== stats || $[21] !== useCallback) {
      t178 = <p className="text-gray-500 text-center py-8">No bookings found</p>;
      $[12] = onCancel;
      $[13] = searchQuery;
      $[14] = sortBy;
      $[15] = sortedBookings;
      $[16] = sortedBookings;
      $[17] = stats;
      $[18] = stats;
      $[19] = stats;
      $[20] = stats;
      $[21] = useCallback;
      $[22] = t339;
    } else {
      t339 = $[22];
    }
  } else {
    const t323 = (booking) => {
      const t8 = () => {
        return toggleExpanded(booking.id);
      };
      if (booking.status === "confirmed") {
        t41 = "bg-green-100";
      } else {
        if (booking.status === "pending") {
          t50 = "bg-yellow-100";
        } else {
          t50 = "bg-red-100";
        }
        t41 = t50;
      }
      t63 = expandedId === booking.id;
      t74 = booking.location;
      t74 = <p className="text-sm">📍 {booking.location}</p>;
      t88 = booking.notes;
      t88 = <p className="text-sm text-gray-600">{booking.notes}</p>;
      const t110 = (a, i) => {
        return <div key={i} className="text-sm flex justify-between"><span>{a.name} ({a.email})</span><span>{a.status}</span></div>;
      };
      t115 = booking.status === "pending";
      const t124 = () => {
        return onConfirm(booking.id);
      };
      t115 = <button onClick={t124} className="text-green-600">Confirm</button>;
      const t130 = () => {
        return onReschedule(booking.id);
      };
      t134 = booking.status !== "cancelled";
      const t143 = () => {
        return handleCancel(booking.id);
      };
      t134 = <button onClick={t143} className="text-red-600">Cancel</button>;
      t63 = <div className="mt-2 pl-4 border-l-2">{t74}{t88}<div className="mt-2"><h4 className="text-xs font-semibold uppercase">Attendees</h4>{booking.attendees.map(t110)}</div><div className="mt-2 flex gap-2">{t115}<button onClick={t130} className="text-blue-600">Reschedule</button>{t134}</div></div>;
      return <li key={booking.id} className="py-3"><div className="flex justify-between items-start cursor-pointer" onClick={t8}><div><h3 className="font-medium">{booking.title}</h3><p className="text-sm text-gray-500">{booking.startTime} - {booking.endTime}</p><p className="text-sm">{booking.attendees.length} attendee(s)</p></div><span className={`px-2 py-1 rounded text-xs ${t41}`}>{booking.status}</span></div>{t63}</li>;
    };
    t178 = <ul className="divide-y">{sortedBookings.map(t323)}</ul>;
  }
  return <div className="space-y-4"><div className="flex justify-between items-center"><div className="flex gap-2 text-sm"><span>{stats.total} total</span><span className="text-green-600">{stats.confirmed} confirmed</span><span className="text-yellow-600">{stats.pending} pending</span><span className="text-red-600">{stats.cancelled} cancelled</span></div><div className="flex gap-2"><input value={searchQuery} onChange={t297} placeholder="Search bookings..." className="border rounded px-2 py-1" /><select value={sortBy} onChange={t303}><option value="date">Sort by date</option><option value="title">Sort by title</option></select></div></div>{t178}</div>;
}

