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
  const $ = _c(41);
  let t9;
  let t15;
  let t21;
  let now;
  let t146;
  let t37;
  let t42;
  let filteredBookings;
  let t147;
  let t48;
  let t51;
  let sortedBookings;
  let t148;
  let t57;
  let t59;
  let t149;
  let t66;
  let t67;
  let t150;
  let t151;
  let t73;
  let t75;
  let t152;
  let t78;
  let t79;
  let t129;
  let t134;
  let { bookings, onCancel, onReschedule, onConfirm, filter } = t0;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t9 = null;
    $[0] = t9;
  } else {
    t9 = $[0];
  }
  let expandedId;
  let setExpandedId;
  ([expandedId, setExpandedId] = useState(t9));
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    t15 = "";
    $[1] = t15;
  } else {
    t15 = $[1];
  }
  let searchQuery;
  let setSearchQuery;
  ([searchQuery, setSearchQuery] = useState(t15));
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    t21 = "date";
    $[2] = t21;
  } else {
    t21 = $[2];
  }
  let sortBy;
  let setSortBy;
  ([sortBy, setSortBy] = useState(t21));
  let t30 = () => {
    return new Date().toISOString();
  };
  let t32 = useMemo(t30, []);
  if ($[3] !== t32 || $[4] !== bookings || $[5] !== filter) {
    now = t32;
    t37 = () => {
      let result;
      result = bookings;
      if (filter === "upcoming") {
        let t8 = (b) => {
          let t1;
          t1 = b.startTime > now;
          t1 = b.status !== "cancelled";
          return t1;
        };
        result = result.filter(t8);
      } else {
        if (filter === "past") {
          let t14 = (b) => {
            return b.endTime < now;
          };
          result = result.filter(t14);
        }
      }
      if (searchQuery) {
        let q;
        q = searchQuery.toLowerCase();
        let t22 = (b) => {
          let t1;
          t1 = b.title.toLowerCase().includes(q);
          let t10 = (a) => {
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
  filteredBookings = t146;
  let t43 = useMemo(t37, t42);
  if ($[10] !== t43) {
    filteredBookings = t43;
    t48 = () => {
      let t3 = (a, b) => {
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
  sortedBookings = t147;
  let t52 = useMemo(t48, t51);
  if ($[15] !== t52) {
    sortedBookings = t52;
    t57 = () => {
      let t4 = (b) => {
        return b.status === "confirmed";
      };
      let t8 = (b) => {
        return b.status === "pending";
      };
      let t12 = (b) => {
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
  let stats = t148;
  stats = useMemo(t57, t59);
  if ($[20] === Symbol.for("react.memo_cache_sentinel")) {
    t66 = (id) => {
      let t3 = (prev) => {
        let t5;
        if (prev === id) {
          t5 = null;
        } else {
          t5 = id;
        }
        return t5;
      };
      let t4 = setExpandedId(t3);
      return undefined;
    };
    t67 = [];
    $[20] = t149;
    $[21] = t66;
    $[22] = t67;
  } else {
    t149 = $[20];
    t66 = $[21];
    t67 = $[22];
  }
  let toggleExpanded = t149;
  let t68 = useCallback(t66, t67);
  if ($[23] !== t68) {
    t150 = t68;
    $[23] = t68;
    $[24] = t150;
  } else {
    t150 = $[24];
  }
  toggleExpanded = t150;
  if ($[25] !== onCancel) {
    t73 = (id) => {
      let t4 = onCancel(id);
      let t8 = setExpandedId(null);
      return undefined;
    };
    t75 = [onCancel];
    $[25] = onCancel;
    $[26] = t151;
    $[27] = t73;
    $[28] = t75;
  } else {
    t151 = $[26];
    t73 = $[27];
    t75 = $[28];
  }
  let handleCancel = t151;
  let t76 = useCallback(t73, t75);
  if ($[29] !== t76) {
    t152 = t76;
    $[29] = t76;
    $[30] = t152;
  } else {
    t152 = $[30];
  }
  handleCancel = t152;
  if ($[31] !== sortedBookings.length || $[32] !== stats.total || $[33] !== stats.confirmed || $[34] !== stats.pending || $[35] !== stats.cancelled) {
    let t112 = (e) => {
      return setSearchQuery(e.target.value);
    };
    let t118 = (e) => {
      return setSortBy(e.target.value);
    };
    if (sortedBookings.length === 0) {
      if ($[36] === Symbol.for("react.memo_cache_sentinel")) {
        t134 = <p className="text-gray-500 text-center py-8">No bookings found</p>;
        $[36] = t134;
      } else {
        t134 = $[36];
      }
    } else {
      let t142 = (booking) => {
        let t7 = () => {
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
        let t82 = (a, i) => {
          return <div key={i} className="text-sm flex justify-between"><span>{a.name} ({a.email})</span><span>{a.status}</span></div>;
        };
        let t87;
        t87 = booking.status === "pending";
        let t93 = () => {
          return onConfirm(booking.id);
        };
        t87 = <button onClick={t93} className="text-green-600">Confirm</button>;
        let t98 = () => {
          return onReschedule(booking.id);
        };
        let t102;
        t102 = booking.status !== "cancelled";
        let t108 = () => {
          return handleCancel(booking.id);
        };
        t102 = <button onClick={t108} className="text-red-600">Cancel</button>;
        t49 = <div className="mt-2 pl-4 border-l-2">{t57}{t66}<div className="mt-2"><h4 className="text-xs font-semibold uppercase">Attendees</h4>{booking.attendees.map(t82)}</div><div className="mt-2 flex gap-2">{t87}<button onClick={t98} className="text-blue-600">Reschedule</button>{t102}</div></div>;
        return <li key={booking.id} className="py-3"><div className="flex justify-between items-start cursor-pointer" onClick={t7}><div><h3 className="font-medium">{booking.title}</h3><p className="text-sm text-gray-500">{booking.startTime} - {booking.endTime}</p><p className="text-sm">{booking.attendees.length} attendee(s)</p></div><span className={`px-2 py-1 rounded text-xs ${t35}`}>{booking.status}</span></div>{t49}</li>;
      };
      t134 = <ul className="divide-y">{sortedBookings.map(t142)}</ul>;
    }
    return <div className="space-y-4"><div className="flex justify-between items-center"><div className="flex gap-2 text-sm"><span>{stats.total} total</span><span className="text-green-600">{stats.confirmed} confirmed</span><span className="text-yellow-600">{stats.pending} pending</span><span className="text-red-600">{stats.cancelled} cancelled</span></div><div className="flex gap-2"><input value={searchQuery} onChange={t112} placeholder="Search bookings..." className="border rounded px-2 py-1" /><select value={sortBy} onChange={t118}><option value="date">Sort by date</option><option value="title">Sort by title</option></select></div></div>{t134}</div>;
    $[31] = sortedBookings.length;
    $[32] = stats.total;
    $[33] = stats.confirmed;
    $[34] = stats.pending;
    $[35] = stats.cancelled;
    $[36] = t78;
    $[37] = t79;
    $[38] = t129;
    $[39] = t134;
  } else {
    t78 = $[36];
    t79 = $[37];
    t129 = $[38];
    t134 = $[39];
  }
}

