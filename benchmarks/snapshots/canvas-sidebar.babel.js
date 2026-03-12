import { c as _c } from "react/compiler-runtime";
// L tier - Inspired by excalidraw Sidebar with layer management
import { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
export function CanvasSidebar(t0) {
  const $ = _c(70);
  const {
    layers,
    activeLayerId,
    onLayerSelect,
    onLayerToggleVisible,
    onLayerToggleLock,
    onLayerRename,
    onLayerReorder,
    onLayerDelete,
    onLayerAdd,
    onLayerDuplicate,
    onLayerOpacity
  } = t0;
  const [activeTab, setActiveTab] = useState("layers");
  const [editingId, setEditingId] = useState(null);
  const [editName, setEditName] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [dragIndex, setDragIndex] = useState(null);
  const [collapsed, setCollapsed] = useState(false);
  const editInputRef = useRef(null);
  let t1;
  let t2;
  if ($[0] !== editingId) {
    t1 = () => {
      if (editingId && editInputRef.current) {
        editInputRef.current.focus();
        editInputRef.current.select();
      }
    };
    t2 = [editingId];
    $[0] = editingId;
    $[1] = t1;
    $[2] = t2;
  } else {
    t1 = $[1];
    t2 = $[2];
  }
  useEffect(t1, t2);
  let t3;
  bb0: {
    if (!searchQuery) {
      t3 = layers;
      break bb0;
    }
    let t4;
    if ($[3] !== layers || $[4] !== searchQuery) {
      const q = searchQuery.toLowerCase();
      t4 = layers.filter(l => l.name.toLowerCase().includes(q));
      $[3] = layers;
      $[4] = searchQuery;
      $[5] = t4;
    } else {
      t4 = $[5];
    }
    t3 = t4;
  }
  const filteredLayers = t3;
  let t4;
  if ($[6] !== activeLayerId || $[7] !== layers) {
    let t5;
    if ($[9] !== activeLayerId) {
      t5 = l_0 => l_0.id === activeLayerId;
      $[9] = activeLayerId;
      $[10] = t5;
    } else {
      t5 = $[10];
    }
    t4 = layers.find(t5);
    $[6] = activeLayerId;
    $[7] = layers;
    $[8] = t4;
  } else {
    t4 = $[8];
  }
  const activeLayer = t4;
  let t5;
  if ($[11] !== layers) {
    t5 = layers.reduce(_temp, 0);
    $[11] = layers;
    $[12] = t5;
  } else {
    t5 = $[12];
  }
  let t6;
  if ($[13] !== layers) {
    t6 = layers.filter(_temp2);
    $[13] = layers;
    $[14] = t6;
  } else {
    t6 = $[14];
  }
  const t7 = t6.length;
  let t8;
  if ($[15] !== layers) {
    t8 = layers.filter(_temp3);
    $[15] = layers;
    $[16] = t8;
  } else {
    t8 = $[16];
  }
  let t9;
  if ($[17] !== t5 || $[18] !== t6.length || $[19] !== t8.length) {
    t9 = {
      totalElements: t5,
      visibleLayers: t7,
      lockedLayers: t8.length
    };
    $[17] = t5;
    $[18] = t6.length;
    $[19] = t8.length;
    $[20] = t9;
  } else {
    t9 = $[20];
  }
  const stats = t9;
  let t10;
  if ($[21] === Symbol.for("react.memo_cache_sentinel")) {
    t10 = layer => {
      setEditingId(layer.id);
      setEditName(layer.name);
    };
    $[21] = t10;
  } else {
    t10 = $[21];
  }
  const startEditing = t10;
  let t11;
  if ($[22] !== editName || $[23] !== editingId || $[24] !== onLayerRename) {
    t11 = () => {
      if (editingId && editName.trim()) {
        onLayerRename(editingId, editName.trim());
      }
      setEditingId(null);
    };
    $[22] = editName;
    $[23] = editingId;
    $[24] = onLayerRename;
    $[25] = t11;
  } else {
    t11 = $[25];
  }
  const commitEdit = t11;
  let t12;
  if ($[26] !== commitEdit) {
    t12 = e => {
      if (e.key === "Enter") {
        commitEdit();
      }
      if (e.key === "Escape") {
        setEditingId(null);
      }
    };
    $[26] = commitEdit;
    $[27] = t12;
  } else {
    t12 = $[27];
  }
  const handleKeyDown = t12;
  let t13;
  if ($[28] === Symbol.for("react.memo_cache_sentinel")) {
    t13 = index => {
      setDragIndex(index);
    };
    $[28] = t13;
  } else {
    t13 = $[28];
  }
  const handleDragStart = t13;
  let t14;
  if ($[29] !== dragIndex || $[30] !== onLayerReorder) {
    t14 = (e_0, index_0) => {
      e_0.preventDefault();
      if (dragIndex !== null && dragIndex !== index_0) {
        onLayerReorder(dragIndex, index_0);
        setDragIndex(index_0);
      }
    };
    $[29] = dragIndex;
    $[30] = onLayerReorder;
    $[31] = t14;
  } else {
    t14 = $[31];
  }
  const handleDragOver = t14;
  let t15;
  if ($[32] === Symbol.for("react.memo_cache_sentinel")) {
    t15 = () => {
      setDragIndex(null);
    };
    $[32] = t15;
  } else {
    t15 = $[32];
  }
  const handleDragEnd = t15;
  if (collapsed) {
    let t16;
    if ($[33] === Symbol.for("react.memo_cache_sentinel")) {
      t16 = /*#__PURE__*/_jsx("div", {
        className: "w-10 bg-white border-l flex flex-col items-center py-2",
        children: /*#__PURE__*/_jsx("button", {
          onClick: () => setCollapsed(false),
          className: "text-gray-500 hover:text-gray-700",
          children: "\u25C0"
        })
      });
      $[33] = t16;
    } else {
      t16 = $[33];
    }
    return t16;
  }
  let t16;
  if ($[34] !== activeTab) {
    t16 = /*#__PURE__*/_jsx("div", {
      className: "flex gap-1",
      children: ["layers", "properties", "history"].map(tab => /*#__PURE__*/_jsx("button", {
        onClick: () => setActiveTab(tab),
        className: `px-2 py-1 text-xs rounded ${activeTab === tab ? "bg-blue-100 text-blue-700" : "text-gray-500 hover:bg-gray-100"}`,
        children: tab.charAt(0).toUpperCase() + tab.slice(1)
      }, tab))
    });
    $[34] = activeTab;
    $[35] = t16;
  } else {
    t16 = $[35];
  }
  let t17;
  if ($[36] === Symbol.for("react.memo_cache_sentinel")) {
    t17 = /*#__PURE__*/_jsx("button", {
      onClick: () => setCollapsed(true),
      className: "text-gray-400 hover:text-gray-600",
      children: "\u25B6"
    });
    $[36] = t17;
  } else {
    t17 = $[36];
  }
  let t18;
  if ($[37] !== t16) {
    t18 = /*#__PURE__*/_jsxs("div", {
      className: "flex items-center justify-between px-3 py-2 border-b",
      children: [t16, t17]
    });
    $[37] = t16;
    $[38] = t18;
  } else {
    t18 = $[38];
  }
  let t19;
  if ($[39] !== activeLayer || $[40] !== activeLayerId || $[41] !== activeTab || $[42] !== commitEdit || $[43] !== dragIndex || $[44] !== editName || $[45] !== editingId || $[46] !== filteredLayers || $[47] !== handleDragOver || $[48] !== handleKeyDown || $[49] !== layers.length || $[50] !== onLayerAdd || $[51] !== onLayerDelete || $[52] !== onLayerDuplicate || $[53] !== onLayerOpacity || $[54] !== onLayerSelect || $[55] !== onLayerToggleLock || $[56] !== onLayerToggleVisible || $[57] !== searchQuery || $[58] !== stats) {
    t19 = activeTab === "layers" && /*#__PURE__*/_jsxs(_Fragment, {
      children: [/*#__PURE__*/_jsx("div", {
        className: "px-3 py-2 border-b",
        children: /*#__PURE__*/_jsx("input", {
          value: searchQuery,
          onChange: e_1 => setSearchQuery(e_1.target.value),
          placeholder: "Search layers...",
          className: "w-full text-xs border rounded px-2 py-1"
        })
      }), /*#__PURE__*/_jsx("div", {
        className: "flex-1 overflow-y-auto",
        children: filteredLayers.map((layer_0, index_1) => /*#__PURE__*/_jsxs("div", {
          draggable: true,
          onDragStart: () => handleDragStart(index_1),
          onDragOver: e_2 => handleDragOver(e_2, index_1),
          onDragEnd: handleDragEnd,
          onClick: () => onLayerSelect(layer_0.id),
          className: `flex items-center px-3 py-2 text-sm cursor-pointer border-b ${layer_0.id === activeLayerId ? "bg-blue-50" : "hover:bg-gray-50"} ${dragIndex === index_1 ? "opacity-50" : ""}`,
          children: [/*#__PURE__*/_jsx("button", {
            onClick: e_3 => {
              e_3.stopPropagation();
              onLayerToggleVisible(layer_0.id);
            },
            className: "mr-2 text-xs",
            children: layer_0.visible ? "\uD83D\uDC41" : "\u25CB"
          }), editingId === layer_0.id ? /*#__PURE__*/_jsx("input", {
            ref: editInputRef,
            value: editName,
            onChange: e_4 => setEditName(e_4.target.value),
            onBlur: commitEdit,
            onKeyDown: handleKeyDown,
            className: "flex-1 text-xs border rounded px-1"
          }) : /*#__PURE__*/_jsx("span", {
            onDoubleClick: () => startEditing(layer_0),
            className: `flex-1 truncate ${!layer_0.visible ? "text-gray-400" : ""}`,
            children: layer_0.name
          }), /*#__PURE__*/_jsx("span", {
            className: "text-xs text-gray-400 ml-1",
            children: layer_0.elements
          }), /*#__PURE__*/_jsx("button", {
            onClick: e_5 => {
              e_5.stopPropagation();
              onLayerToggleLock(layer_0.id);
            },
            className: "ml-1 text-xs",
            children: layer_0.locked ? "\uD83D\uDD12" : "\uD83D\uDD13"
          })]
        }, layer_0.id))
      }), /*#__PURE__*/_jsxs("div", {
        className: "px-3 py-2 border-t",
        children: [/*#__PURE__*/_jsxs("div", {
          className: "flex justify-between items-center",
          children: [/*#__PURE__*/_jsxs("div", {
            className: "text-xs text-gray-500",
            children: [stats.totalElements, " elements \xB7 ", stats.visibleLayers, "/", layers.length, " visible"]
          }), /*#__PURE__*/_jsxs("div", {
            className: "flex gap-1",
            children: [/*#__PURE__*/_jsx("button", {
              onClick: onLayerAdd,
              className: "text-xs px-2 py-1 bg-blue-500 text-white rounded",
              children: "+"
            }), activeLayer && /*#__PURE__*/_jsxs(_Fragment, {
              children: [/*#__PURE__*/_jsx("button", {
                onClick: () => onLayerDuplicate(activeLayerId),
                className: "text-xs px-2 py-1 border rounded",
                children: "\u29C9"
              }), /*#__PURE__*/_jsx("button", {
                onClick: () => onLayerDelete(activeLayerId),
                className: "text-xs px-2 py-1 border rounded text-red-500",
                disabled: layers.length <= 1,
                children: "\uD83D\uDDD1"
              })]
            })]
          })]
        }), activeLayer && /*#__PURE__*/_jsxs("div", {
          className: "mt-2",
          children: [/*#__PURE__*/_jsx("label", {
            className: "text-xs text-gray-500",
            children: "Opacity"
          }), /*#__PURE__*/_jsx("input", {
            type: "range",
            min: 0,
            max: 100,
            value: activeLayer.opacity * 100,
            onChange: e_6 => onLayerOpacity(activeLayerId, parseInt(e_6.target.value) / 100),
            className: "w-full"
          }), /*#__PURE__*/_jsxs("span", {
            className: "text-xs text-gray-400",
            children: [Math.round(activeLayer.opacity * 100), "%"]
          })]
        })]
      })]
    });
    $[39] = activeLayer;
    $[40] = activeLayerId;
    $[41] = activeTab;
    $[42] = commitEdit;
    $[43] = dragIndex;
    $[44] = editName;
    $[45] = editingId;
    $[46] = filteredLayers;
    $[47] = handleDragOver;
    $[48] = handleKeyDown;
    $[49] = layers.length;
    $[50] = onLayerAdd;
    $[51] = onLayerDelete;
    $[52] = onLayerDuplicate;
    $[53] = onLayerOpacity;
    $[54] = onLayerSelect;
    $[55] = onLayerToggleLock;
    $[56] = onLayerToggleVisible;
    $[57] = searchQuery;
    $[58] = stats;
    $[59] = t19;
  } else {
    t19 = $[59];
  }
  let t20;
  if ($[60] !== activeLayer || $[61] !== activeTab) {
    t20 = activeTab === "properties" && /*#__PURE__*/_jsx("div", {
      className: "p-3 text-sm text-gray-500",
      children: activeLayer ? /*#__PURE__*/_jsxs("div", {
        className: "space-y-2",
        children: [/*#__PURE__*/_jsxs("div", {
          children: [/*#__PURE__*/_jsx("strong", {
            children: "Name:"
          }), " ", activeLayer.name]
        }), /*#__PURE__*/_jsxs("div", {
          children: [/*#__PURE__*/_jsx("strong", {
            children: "Elements:"
          }), " ", activeLayer.elements]
        }), /*#__PURE__*/_jsxs("div", {
          children: [/*#__PURE__*/_jsx("strong", {
            children: "Visible:"
          }), " ", activeLayer.visible ? "Yes" : "No"]
        }), /*#__PURE__*/_jsxs("div", {
          children: [/*#__PURE__*/_jsx("strong", {
            children: "Locked:"
          }), " ", activeLayer.locked ? "Yes" : "No"]
        }), /*#__PURE__*/_jsxs("div", {
          children: [/*#__PURE__*/_jsx("strong", {
            children: "Opacity:"
          }), " ", Math.round(activeLayer.opacity * 100), "%"]
        })]
      }) : /*#__PURE__*/_jsx("p", {
        children: "Select a layer to view properties"
      })
    });
    $[60] = activeLayer;
    $[61] = activeTab;
    $[62] = t20;
  } else {
    t20 = $[62];
  }
  let t21;
  if ($[63] !== activeTab) {
    t21 = activeTab === "history" && /*#__PURE__*/_jsx("div", {
      className: "p-3 text-sm text-gray-500 text-center",
      children: "History panel (not implemented)"
    });
    $[63] = activeTab;
    $[64] = t21;
  } else {
    t21 = $[64];
  }
  let t22;
  if ($[65] !== t18 || $[66] !== t19 || $[67] !== t20 || $[68] !== t21) {
    t22 = /*#__PURE__*/_jsxs("div", {
      className: "w-64 bg-white border-l flex flex-col h-full",
      children: [t18, t19, t20, t21]
    });
    $[65] = t18;
    $[66] = t19;
    $[67] = t20;
    $[68] = t21;
    $[69] = t22;
  } else {
    t22 = $[69];
  }
  return t22;
}
function _temp3(l_3) {
  return l_3.locked;
}
function _temp2(l_2) {
  return l_2.visible;
}
function _temp(sum, l_1) {
  return sum + l_1.elements;
}