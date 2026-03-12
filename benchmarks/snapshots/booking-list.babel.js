import { c as _c } from "react/compiler-runtime";
// M tier - Inspired by cal.com BookingListItem with scheduling logic
import { useState, useMemo, useCallback } from 'react';
import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
export function BookingList(t0) {
  const $ = _c(62);
  const {
    bookings,
    onCancel,
    onReschedule,
    onConfirm,
    filter: t1
  } = t0;
  const filter = t1 === undefined ? "all" : t1;
  const [expandedId, setExpandedId] = useState(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [sortBy, setSortBy] = useState("date");
  let t2;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t2 = new Date().toISOString();
    $[0] = t2;
  } else {
    t2 = $[0];
  }
  const now = t2;
  let result;
  if ($[1] !== bookings || $[2] !== filter || $[3] !== searchQuery) {
    result = bookings;
    if (filter === "upcoming") {
      let t3;
      if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
        t3 = b => b.startTime > now && b.status !== "cancelled";
        $[5] = t3;
      } else {
        t3 = $[5];
      }
      result = result.filter(t3);
    } else {
      if (filter === "past") {
        let t3;
        if ($[6] === Symbol.for("react.memo_cache_sentinel")) {
          t3 = b_0 => b_0.endTime < now;
          $[6] = t3;
        } else {
          t3 = $[6];
        }
        result = result.filter(t3);
      }
    }
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(b_1 => b_1.title.toLowerCase().includes(q) || b_1.attendees.some(a => a.name.toLowerCase().includes(q) || a.email.toLowerCase().includes(q)));
    }
    $[1] = bookings;
    $[2] = filter;
    $[3] = searchQuery;
    $[4] = result;
  } else {
    result = $[4];
  }
  const filteredBookings = result;
  let t3;
  if ($[7] !== filteredBookings || $[8] !== sortBy) {
    let t4;
    if ($[10] !== sortBy) {
      t4 = (a_0, b_2) => {
        if (sortBy === "date") {
          return a_0.startTime.localeCompare(b_2.startTime);
        }
        return a_0.title.localeCompare(b_2.title);
      };
      $[10] = sortBy;
      $[11] = t4;
    } else {
      t4 = $[11];
    }
    t3 = [...filteredBookings].sort(t4);
    $[7] = filteredBookings;
    $[8] = sortBy;
    $[9] = t3;
  } else {
    t3 = $[9];
  }
  const sortedBookings = t3;
  const t4 = sortedBookings.length;
  let t5;
  if ($[12] !== sortedBookings) {
    t5 = sortedBookings.filter(_temp);
    $[12] = sortedBookings;
    $[13] = t5;
  } else {
    t5 = $[13];
  }
  const t6 = t5.length;
  let t7;
  if ($[14] !== sortedBookings) {
    t7 = sortedBookings.filter(_temp2);
    $[14] = sortedBookings;
    $[15] = t7;
  } else {
    t7 = $[15];
  }
  const t8 = t7.length;
  let t9;
  if ($[16] !== sortedBookings) {
    t9 = sortedBookings.filter(_temp3);
    $[16] = sortedBookings;
    $[17] = t9;
  } else {
    t9 = $[17];
  }
  let t10;
  if ($[18] !== sortedBookings.length || $[19] !== t5.length || $[20] !== t7.length || $[21] !== t9.length) {
    t10 = {
      total: t4,
      confirmed: t6,
      pending: t8,
      cancelled: t9.length
    };
    $[18] = sortedBookings.length;
    $[19] = t5.length;
    $[20] = t7.length;
    $[21] = t9.length;
    $[22] = t10;
  } else {
    t10 = $[22];
  }
  const stats = t10;
  let t11;
  if ($[23] === Symbol.for("react.memo_cache_sentinel")) {
    t11 = id => {
      setExpandedId(prev => prev === id ? null : id);
    };
    $[23] = t11;
  } else {
    t11 = $[23];
  }
  const toggleExpanded = t11;
  let t12;
  if ($[24] !== onCancel) {
    t12 = id_0 => {
      onCancel(id_0);
      setExpandedId(null);
    };
    $[24] = onCancel;
    $[25] = t12;
  } else {
    t12 = $[25];
  }
  const handleCancel = t12;
  let t13;
  if ($[26] !== stats.total) {
    t13 = /*#__PURE__*/_jsxs("span", {
      children: [stats.total, " total"]
    });
    $[26] = stats.total;
    $[27] = t13;
  } else {
    t13 = $[27];
  }
  let t14;
  if ($[28] !== stats.confirmed) {
    t14 = /*#__PURE__*/_jsxs("span", {
      className: "text-green-600",
      children: [stats.confirmed, " confirmed"]
    });
    $[28] = stats.confirmed;
    $[29] = t14;
  } else {
    t14 = $[29];
  }
  let t15;
  if ($[30] !== stats.pending) {
    t15 = /*#__PURE__*/_jsxs("span", {
      className: "text-yellow-600",
      children: [stats.pending, " pending"]
    });
    $[30] = stats.pending;
    $[31] = t15;
  } else {
    t15 = $[31];
  }
  let t16;
  if ($[32] !== stats.cancelled) {
    t16 = /*#__PURE__*/_jsxs("span", {
      className: "text-red-600",
      children: [stats.cancelled, " cancelled"]
    });
    $[32] = stats.cancelled;
    $[33] = t16;
  } else {
    t16 = $[33];
  }
  let t17;
  if ($[34] !== t13 || $[35] !== t14 || $[36] !== t15 || $[37] !== t16) {
    t17 = /*#__PURE__*/_jsxs("div", {
      className: "flex gap-2 text-sm",
      children: [t13, t14, t15, t16]
    });
    $[34] = t13;
    $[35] = t14;
    $[36] = t15;
    $[37] = t16;
    $[38] = t17;
  } else {
    t17 = $[38];
  }
  let t18;
  if ($[39] === Symbol.for("react.memo_cache_sentinel")) {
    t18 = e => setSearchQuery(e.target.value);
    $[39] = t18;
  } else {
    t18 = $[39];
  }
  let t19;
  if ($[40] !== searchQuery) {
    t19 = /*#__PURE__*/_jsx("input", {
      value: searchQuery,
      onChange: t18,
      placeholder: "Search bookings...",
      className: "border rounded px-2 py-1"
    });
    $[40] = searchQuery;
    $[41] = t19;
  } else {
    t19 = $[41];
  }
  let t20;
  let t21;
  let t22;
  if ($[42] === Symbol.for("react.memo_cache_sentinel")) {
    t20 = e_0 => setSortBy(e_0.target.value);
    t21 = /*#__PURE__*/_jsx("option", {
      value: "date",
      children: "Sort by date"
    });
    t22 = /*#__PURE__*/_jsx("option", {
      value: "title",
      children: "Sort by title"
    });
    $[42] = t20;
    $[43] = t21;
    $[44] = t22;
  } else {
    t20 = $[42];
    t21 = $[43];
    t22 = $[44];
  }
  let t23;
  if ($[45] !== sortBy) {
    t23 = /*#__PURE__*/_jsxs("select", {
      value: sortBy,
      onChange: t20,
      children: [t21, t22]
    });
    $[45] = sortBy;
    $[46] = t23;
  } else {
    t23 = $[46];
  }
  let t24;
  if ($[47] !== t19 || $[48] !== t23) {
    t24 = /*#__PURE__*/_jsxs("div", {
      className: "flex gap-2",
      children: [t19, t23]
    });
    $[47] = t19;
    $[48] = t23;
    $[49] = t24;
  } else {
    t24 = $[49];
  }
  let t25;
  if ($[50] !== t17 || $[51] !== t24) {
    t25 = /*#__PURE__*/_jsxs("div", {
      className: "flex justify-between items-center",
      children: [t17, t24]
    });
    $[50] = t17;
    $[51] = t24;
    $[52] = t25;
  } else {
    t25 = $[52];
  }
  let t26;
  if ($[53] !== expandedId || $[54] !== handleCancel || $[55] !== onConfirm || $[56] !== onReschedule || $[57] !== sortedBookings) {
    t26 = sortedBookings.length === 0 ? /*#__PURE__*/_jsx("p", {
      className: "text-gray-500 text-center py-8",
      children: "No bookings found"
    }) : /*#__PURE__*/_jsx("ul", {
      className: "divide-y",
      children: sortedBookings.map(booking => /*#__PURE__*/_jsxs("li", {
        className: "py-3",
        children: [/*#__PURE__*/_jsxs("div", {
          className: "flex justify-between items-start cursor-pointer",
          onClick: () => toggleExpanded(booking.id),
          children: [/*#__PURE__*/_jsxs("div", {
            children: [/*#__PURE__*/_jsx("h3", {
              className: "font-medium",
              children: booking.title
            }), /*#__PURE__*/_jsxs("p", {
              className: "text-sm text-gray-500",
              children: [booking.startTime, " - ", booking.endTime]
            }), /*#__PURE__*/_jsxs("p", {
              className: "text-sm",
              children: [booking.attendees.length, " attendee(s)"]
            })]
          }), /*#__PURE__*/_jsx("span", {
            className: `px-2 py-1 rounded text-xs ${booking.status === "confirmed" ? "bg-green-100" : booking.status === "pending" ? "bg-yellow-100" : "bg-red-100"}`,
            children: booking.status
          })]
        }), expandedId === booking.id && /*#__PURE__*/_jsxs("div", {
          className: "mt-2 pl-4 border-l-2",
          children: [booking.location && /*#__PURE__*/_jsxs("p", {
            className: "text-sm",
            children: ["\uD83D\uDCCD ", booking.location]
          }), booking.notes && /*#__PURE__*/_jsx("p", {
            className: "text-sm text-gray-600",
            children: booking.notes
          }), /*#__PURE__*/_jsxs("div", {
            className: "mt-2",
            children: [/*#__PURE__*/_jsx("h4", {
              className: "text-xs font-semibold uppercase",
              children: "Attendees"
            }), booking.attendees.map(_temp4)]
          }), /*#__PURE__*/_jsxs("div", {
            className: "mt-2 flex gap-2",
            children: [booking.status === "pending" && /*#__PURE__*/_jsx("button", {
              onClick: () => onConfirm(booking.id),
              className: "text-green-600",
              children: "Confirm"
            }), /*#__PURE__*/_jsx("button", {
              onClick: () => onReschedule(booking.id),
              className: "text-blue-600",
              children: "Reschedule"
            }), booking.status !== "cancelled" && /*#__PURE__*/_jsx("button", {
              onClick: () => handleCancel(booking.id),
              className: "text-red-600",
              children: "Cancel"
            })]
          })]
        })]
      }, booking.id))
    });
    $[53] = expandedId;
    $[54] = handleCancel;
    $[55] = onConfirm;
    $[56] = onReschedule;
    $[57] = sortedBookings;
    $[58] = t26;
  } else {
    t26 = $[58];
  }
  let t27;
  if ($[59] !== t25 || $[60] !== t26) {
    t27 = /*#__PURE__*/_jsxs("div", {
      className: "space-y-4",
      children: [t25, t26]
    });
    $[59] = t25;
    $[60] = t26;
    $[61] = t27;
  } else {
    t27 = $[61];
  }
  return t27;
}
function _temp4(a_1, i) {
  return /*#__PURE__*/_jsxs("div", {
    className: "text-sm flex justify-between",
    children: [/*#__PURE__*/_jsxs("span", {
      children: [a_1.name, " (", a_1.email, ")"]
    }), /*#__PURE__*/_jsx("span", {
      children: a_1.status
    })]
  }, i);
}
function _temp3(b_5) {
  return b_5.status === "cancelled";
}
function _temp2(b_4) {
  return b_4.status === "pending";
}
function _temp(b_3) {
  return b_3.status === "confirmed";
}