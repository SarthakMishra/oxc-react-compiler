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
  const $ = _c(45);
  const { bookings, onCancel, onReschedule, onConfirm, filter } = t0;
  let t9;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    $[0] = t9;
  } else {
    t9 = $[0];
  }
  let t15;
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    $[1] = t15;
  } else {
    t15 = $[1];
  }
  let t21;
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    $[2] = t21;
  } else {
    t21 = $[2];
  }
  let now;
  const t30 = () => {
    return new Date().toISOString();
  };
  const t32 = useMemo(t30, []);
  let t146;
  let t37;
  let t42;
  if ($[3] !== t32 || $[4] !== bookings || $[5] !== filter) {
    now = t32;
    t37 = () => {
      let result;
      result = bookings;
      if (filter === "upcoming") {
        const t8 = (b) => {
          let t1;
          t1 = b.startTime > now;
          t1 = b.status !== "cancelled";
          return t1;
        };
        result = result.filter(t8);
      } else {
        if (filter === "past") {
          const t14 = (b) => {
            return b.endTime < now;
          };
          result = result.filter(t14);
        }
      }
      if (searchQuery) {
        let q;
        q = searchQuery.toLowerCase();
        const t22 = (b) => {
          let t1;
          t1 = b.title.toLowerCase().includes(q);
          const t10 = (a) => {
            let t1;
            t1 = a.name.toLowerCase().includes(q);
            t1 = a.email.toLowerCase().includes(q);
            return t1;
          };
          t1 = b.attendees.some(t10);
          return t1;
        };
        result = result.filter(t22);
      }
      return result;
    };
    t42 = [bookings, filter, now, searchQuery];
    $[3] = t32;
    $[4] = bookings;
    $[5] = filter;
    $[6] = now;
    $[7] = t146;
    $[8] = t37;
    $[9] = t42;
  } else {
    now = $[6];
    t146 = $[7];
    t37 = $[8];
    t42 = $[9];
  }
  const filteredBookings = t146;
  const t43 = useMemo(t37, t42);
  let t147;
  let filteredBookings;
  let t48;
  let t51;
  if ($[10] !== t43) {
    filteredBookings = t43;
    t48 = () => {
      const t3 = (a, b) => {
        if (sortBy === "date") {
          return a.startTime.localeCompare(b.startTime);
        }
        return a.title.localeCompare(b.title);
      };
      return [...filteredBookings].sort(t3);
    };
    t51 = [filteredBookings, sortBy];
    $[10] = t43;
    $[11] = filteredBookings;
    $[12] = t147;
    $[13] = t48;
    $[14] = t51;
  } else {
    filteredBookings = $[11];
    t147 = $[12];
    t48 = $[13];
    t51 = $[14];
  }
  const sortedBookings = t147;
  const t52 = useMemo(t48, t51);
  let t148;
  let sortedBookings;
  let t57;
  let t59;
  if ($[15] !== t52) {
    sortedBookings = t52;
    t57 = () => {
      const t4 = (b) => {
        return b.status === "confirmed";
      };
      const t8 = (b) => {
        return b.status === "pending";
      };
      const t12 = (b) => {
        return b.status === "cancelled";
      };
      return { total: sortedBookings.length, confirmed: sortedBookings.filter(t4).length, pending: sortedBookings.filter(t8).length, cancelled: sortedBookings.filter(t12).length };
    };
    t59 = [sortedBookings];
    $[15] = t52;
    $[16] = sortedBookings;
    $[17] = t148;
    $[18] = t57;
    $[19] = t59;
  } else {
    sortedBookings = $[16];
    t148 = $[17];
    t57 = $[18];
    t59 = $[19];
  }
  const stats = t148;
  const t60 = useMemo(t57, t59);
  let t149;
  if ($[20] !== t60) {
    t149 = t60;
    $[20] = t60;
    $[21] = t149;
  } else {
    t149 = $[21];
  }
  const stats = t149;
  let t150;
  let t66;
  let t67;
  if ($[22] === Symbol.for("react.memo_cache_sentinel")) {
    t66 = (id) => {
      const t3 = (prev) => {
        let t5;
        if (prev === id) {
          t5 = null;
        } else {
          t5 = id;
        }
        return t5;
      };
      const t4 = setExpandedId(t3);
      return undefined;
    };
    t67 = [];
    $[22] = t150;
    $[23] = t66;
    $[24] = t67;
  } else {
    t150 = $[22];
    t66 = $[23];
    t67 = $[24];
  }
  const toggleExpanded = t150;
  const t68 = useCallback(t66, t67);
  let t151;
  if ($[25] !== t68) {
    t151 = t68;
    $[25] = t68;
    $[26] = t151;
  } else {
    t151 = $[26];
  }
  const toggleExpanded = t151;
  let t152;
  let t73;
  let t75;
  if ($[27] !== onCancel) {
    t73 = (id) => {
      const t4 = onCancel(id);
      const t8 = setExpandedId(null);
      return undefined;
    };
    t75 = [onCancel];
    $[27] = onCancel;
    $[28] = t152;
    $[29] = t73;
    $[30] = t75;
  } else {
    t152 = $[28];
    t73 = $[29];
    t75 = $[30];
  }
  const handleCancel = t152;
  const t76 = useCallback(t73, t75);
  let t153;
  if ($[31] !== t76) {
    t153 = t76;
    $[31] = t76;
    $[32] = t153;
  } else {
    t153 = $[32];
  }
  const handleCancel = t153;
  const t112 = (e) => {
    return setSearchQuery(e.target.value);
  };
  const t118 = (e) => {
    return setSortBy(e.target.value);
  };
  let t134;
  if (sortedBookings.length === 0) {
    let t145;
    let t154;
    let t155;
    let t30;
    let t31;
    let t156;
    if ($[33] !== t43 || $[34] !== sortedBookings.length || $[35] !== stats.total || $[36] !== stats.confirmed || $[37] !== stats.pending || $[38] !== stats.cancelled) {
      t134 = <p className="text-gray-500 text-center py-8">No bookings found</p>;
      $[33] = t43;
      $[34] = sortedBookings.length;
      $[35] = stats.total;
      $[36] = stats.confirmed;
      $[37] = stats.pending;
      $[38] = stats.cancelled;
      $[39] = t145;
      $[40] = t154;
      $[41] = t155;
      $[42] = t30;
      $[43] = t31;
      $[44] = t156;
    } else {
      t145 = $[39];
      t154 = $[40];
      t155 = $[41];
      t30 = $[42];
      t31 = $[43];
      t156 = $[44];
    }
    const sortBy = t154;
    now = t155;
    const searchQuery = t156;
  } else {
    const t142 = (booking) => {
      const t7 = () => {
        return toggleExpanded(booking.id);
      };
      let t35;
      if (booking.status === "confirmed") {
        t35 = "bg-green-100";
      } else {
        let t41;
        if (booking.status === "pending") {
          t41 = "bg-yellow-100";
        } else {
          t41 = "bg-red-100";
        }
        t35 = t41;
      }
      let t49;
      t49 = expandedId === booking.id;
      let t57;
      t57 = booking.location;
      t57 = <p className="text-sm">📍 {booking.location}</p>;
      let t66;
      t66 = booking.notes;
      t66 = <p className="text-sm text-gray-600">{booking.notes}</p>;
      const t82 = (a, i) => {
        return <div key={i} className="text-sm flex justify-between"><span>{a.name} ({a.email})</span><span>{a.status}</span></div>;
      };
      let t87;
      t87 = booking.status === "pending";
      const t93 = () => {
        return onConfirm(booking.id);
      };
      t87 = <button onClick={t93} className="text-green-600">Confirm</button>;
      const t98 = () => {
        return onReschedule(booking.id);
      };
      let t102;
      t102 = booking.status !== "cancelled";
      const t108 = () => {
        return handleCancel(booking.id);
      };
      t102 = <button onClick={t108} className="text-red-600">Cancel</button>;
      t49 = <div className="mt-2 pl-4 border-l-2">{t57}{t66}<div className="mt-2"><h4 className="text-xs font-semibold uppercase">Attendees</h4>{booking.attendees.map(t82)}</div><div className="mt-2 flex gap-2">{t87}<button onClick={t98} className="text-blue-600">Reschedule</button>{t102}</div></div>;
      return <li key={booking.id} className="py-3"><div className="flex justify-between items-start cursor-pointer" onClick={t7}><div><h3 className="font-medium">{booking.title}</h3><p className="text-sm text-gray-500">{booking.startTime} - {booking.endTime}</p><p className="text-sm">{booking.attendees.length} attendee(s)</p></div><span className={`px-2 py-1 rounded text-xs ${t35}`}>{booking.status}</span></div>{t49}</li>;
    };
    t134 = <ul className="divide-y">{sortedBookings.map(t142)}</ul>;
  }
  return <div className="space-y-4"><div className="flex justify-between items-center"><div className="flex gap-2 text-sm"><span>{stats.total} total</span><span className="text-green-600">{stats.confirmed} confirmed</span><span className="text-yellow-600">{stats.pending} pending</span><span className="text-red-600">{stats.cancelled} cancelled</span></div><div className="flex gap-2"><input value={searchQuery} onChange={t112} placeholder="Search bookings..." className="border rounded px-2 py-1" /><select value={sortBy} onChange={t118}><option value="date">Sort by date</option><option value="title">Sort by title</option></select></div></div>{t134}</div>;
}

