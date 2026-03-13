import { c as _c } from "react/compiler-runtime";
// L tier - Inspired by excalidraw Sidebar with layer management
import { useState, useMemo, useCallback, useRef, useEffect } from 'react';

interface Layer {
  id: string;
  name: string;
  visible: boolean;
  locked: boolean;
  opacity: number;
  elements: number;
}

interface SidebarProps {
  layers: Layer[];
  activeLayerId: string;
  onLayerSelect: (id: string) => void;
  onLayerToggleVisible: (id: string) => void;
  onLayerToggleLock: (id: string) => void;
  onLayerRename: (id: string, name: string) => void;
  onLayerReorder: (fromIndex: number, toIndex: number) => void;
  onLayerDelete: (id: string) => void;
  onLayerAdd: () => void;
  onLayerDuplicate: (id: string) => void;
  onLayerOpacity: (id: string, opacity: number) => void;
}

type Tab = 'layers' | 'properties' | 'history';

export function CanvasSidebar(t0) {
  const $ = _c(50);
  const { layers, activeLayerId, onLayerSelect, onLayerToggleVisible, onLayerToggleLock, onLayerRename, onLayerReorder, onLayerDelete, onLayerAdd, onLayerDuplicate, onLayerOpacity } = t0;
  const editInputRef = useRef(null);
  let filteredLayers;
  if ($[0] !== useEffect || $[1] !== editingId) {
    const t495 = () => {
      t0 = editingId;
      t0 = editInputRef.current;
      if (t0) {
        const t12 = editInputRef.current.focus();
        const t16 = editInputRef.current.select();
      }
      return undefined;
    };
    const t498 = useEffect(t495, [editingId]);
    $[0] = useEffect;
    $[1] = editingId;
  }
  let activeLayer;
  if ($[2] !== layers || $[3] !== searchQuery || $[4] !== useMemo) {
    const t501 = () => {
      if (!searchQuery) {
        return layers;
      }
      const q = searchQuery.toLowerCase();
      const t13 = (l) => {
        return l.name.toLowerCase().includes(q);
      };
      return layers.filter(t13);
    };
    filteredLayers = useMemo(t501, [layers, searchQuery]);
    $[2] = layers;
    $[3] = searchQuery;
    $[4] = useMemo;
  }
  let stats;
  if ($[5] !== activeLayerId || $[6] !== layers || $[7] !== useMemo) {
    const t509 = () => {
      const t2 = (l) => {
        return l.id === activeLayerId;
      };
      return layers.find(t2);
    };
    activeLayer = useMemo(t509, [layers, activeLayerId]);
    $[5] = activeLayerId;
    $[6] = layers;
    $[7] = useMemo;
  }
  let startEditing;
  if ($[8] !== layers || $[9] !== useMemo) {
    const t517 = () => {
      const t2 = (sum, l) => {
        return sum + l.elements;
      };
      const t7 = (l) => {
        return l.visible;
      };
      const t12 = (l) => {
        return l.locked;
      };
      return { totalElements: layers.reduce(t2, 0), visibleLayers: layers.filter(t7).length, lockedLayers: layers.filter(t12).length };
    };
    stats = useMemo(t517, [layers]);
    $[8] = layers;
    $[9] = useMemo;
  }
  let commitEdit;
  if ($[10] !== useCallback) {
    const t524 = (layer) => {
      const t6 = setEditingId(layer.id);
      const t12 = setEditName(layer.name);
      return undefined;
    };
    startEditing = useCallback(t524, []);
    $[10] = useCallback;
  }
  let handleKeyDown;
  if ($[11] !== editName || $[12] !== editingId || $[13] !== onLayerRename || $[14] !== useCallback) {
    const t530 = () => {
      t0 = editingId;
      t0 = editName.trim();
      if (t0) {
        const t16 = onLayerRename(editingId, editName.trim());
      }
      const t20 = setEditingId(null);
      return undefined;
    };
    commitEdit = useCallback(t530, [editingId, editName, onLayerRename]);
    $[11] = editName;
    $[12] = editingId;
    $[13] = onLayerRename;
    $[14] = useCallback;
  }
  let handleDragStart;
  if ($[15] !== commitEdit || $[16] !== useCallback) {
    const t539 = (e) => {
      if (e.key === "Enter") {
        const t8 = commitEdit();
      }
      if (e.key === "Escape") {
        const t17 = setEditingId(null);
      }
      return undefined;
    };
    handleKeyDown = useCallback(t539, [commitEdit]);
    $[15] = commitEdit;
    $[16] = useCallback;
  }
  let handleDragOver;
  if ($[17] !== useCallback) {
    const t546 = (index) => {
      const t5 = setDragIndex(index);
      return undefined;
    };
    handleDragStart = useCallback(t546, []);
    $[17] = useCallback;
  }
  let handleDragEnd;
  if ($[18] !== dragIndex || $[19] !== onLayerReorder || $[20] !== useCallback) {
    const t552 = (e, index) => {
      const t4 = e.preventDefault();
      t5 = dragIndex !== null;
      t5 = dragIndex !== index;
      if (t5) {
        const t24 = onLayerReorder(dragIndex, index);
        const t29 = setDragIndex(index);
      }
      return undefined;
    };
    handleDragOver = useCallback(t552, [dragIndex, onLayerReorder]);
    $[18] = dragIndex;
    $[19] = onLayerReorder;
    $[20] = useCallback;
  }
  const t560 = () => {
    const t3 = setDragIndex(null);
    return undefined;
  };
  handleDragEnd = useCallback(t560, []);
  if (collapsed) {
    let t1044;
    let t564;
    if ($[21] !== activeLayer || $[22] !== activeLayer || $[23] !== activeLayer || $[24] !== activeLayer || $[25] !== activeTab || $[26] !== collapsed || $[27] !== filteredLayers || $[28] !== layers || $[29] !== layers || $[30] !== onLayerAdd || $[31] !== searchQuery || $[32] !== stats || $[33] !== stats || $[34] !== useCallback) {
      const t1040 = () => {
        return setCollapsed(false);
      };
      t1044 = <div className="w-10 bg-white border-l flex flex-col items-center py-2"><button onClick={t1040} className="text-gray-500 hover:text-gray-700">\n          ◀\n        </button></div>;
      $[21] = activeLayer;
      $[22] = activeLayer;
      $[23] = activeLayer;
      $[24] = activeLayer;
      $[25] = activeTab;
      $[26] = collapsed;
      $[27] = filteredLayers;
      $[28] = layers;
      $[29] = layers;
      $[30] = onLayerAdd;
      $[31] = searchQuery;
      $[32] = stats;
      $[33] = stats;
      $[34] = useCallback;
      $[35] = t1044;
      $[36] = t564;
    } else {
      t1044 = $[35];
      t564 = $[36];
    }
    return t1044;
  }
  let t1031;
  if ($[37] !== t233 || $[38] !== t347 || $[39] !== activeTab) {
    const t575 = (tab) => {
      const t4 = () => {
        return setActiveTab(tab);
      };
      if (activeTab === tab) {
        t10 = "bg-blue-100 text-blue-700";
      } else {
        t10 = "text-gray-500 hover:bg-gray-100";
      }
      return <button key={tab} onClick={t4} className={`px-2 py-1 text-xs rounded ${t10}`}>{tab.charAt(0).toUpperCase() + tab.slice(1)}</button>;
    };
    const t579 = () => {
      return setCollapsed(true);
    };
    $[37] = t233;
    $[38] = t347;
    $[39] = activeTab;
    $[40] = t1031;
  } else {
    t1031 = $[40];
  }
  t233 = activeTab === "layers";
  const t593 = (e) => {
    return setSearchQuery(e.target.value);
  };
  const t601 = (layer, index) => {
    const t7 = () => {
      return handleDragStart(index);
    };
    const t8 = (e) => {
      return handleDragOver(e, index);
    };
    const t11 = () => {
      return onLayerSelect(layer.id);
    };
    if (layer.id === activeLayerId) {
      t18 = "bg-blue-50";
    } else {
      t18 = "hover:bg-gray-50";
    }
    if (dragIndex === index) {
      t29 = "opacity-50";
    } else {
      t29 = "";
    }
    const t37 = (e) => {
      const t3 = e.stopPropagation();
      const t9 = onLayerToggleVisible(layer.id);
      return undefined;
    };
    if (layer.visible) {
      t42 = "👁";
    } else {
      t42 = "○";
    }
    if (editingId === layer.id) {
      const t62 = (e) => {
        return setEditName(e.target.value);
      };
      t55 = <input ref={editInputRef} value={editName} onChange={t62} onBlur={commitEdit} onKeyDown={handleKeyDown} className="flex-1 text-xs border rounded px-1" />;
    } else {
      const t71 = () => {
        return startEditing(layer);
      };
      if (!layer.visible) {
        t76 = "text-gray-400";
      } else {
        t76 = "";
      }
      t55 = <span onDoubleClick={t71} className={`flex-1 truncate ${t76}`}>{layer.name}</span>;
    }
    const t95 = (e) => {
      const t3 = e.stopPropagation();
      const t9 = onLayerToggleLock(layer.id);
      return undefined;
    };
    if (layer.locked) {
      t100 = "🔒";
    } else {
      t100 = "🔓";
    }
    return <div key={layer.id} draggable onDragStart={t7} onDragOver={t8} onDragEnd={handleDragEnd} onClick={t11} className={`flex items-center px-3 py-2 text-sm cursor-pointer border-b ${t18} ${t29}`}><button onClick={t37} className="mr-2 text-xs">{t42}</button>{t55}<span className="text-xs text-gray-400 ml-1">{layer.elements}</span><button onClick={t95} className="ml-1 text-xs">{t100}</button></div>;
  };
  t284 = activeLayer;
  const t631 = () => {
    return onLayerDuplicate(activeLayerId);
  };
  const t636 = () => {
    return onLayerDelete(activeLayerId);
  };
  t284 = <><button onClick={t631} className="text-xs px-2 py-1 border rounded">\n                      ⧉\n                    </button><button onClick={t636} className="text-xs px-2 py-1 border rounded text-red-500" disabled={layers.length <= 1}>\n                      🗑\n                    </button></>;
  t308 = activeLayer;
  const t681 = (e) => {
    return onLayerOpacity(activeLayerId, parseInt(e.target.value) / 100);
  };
  t308 = <div className="mt-2"><label className="text-xs text-gray-500">Opacity</label><input type="range" min={0} max={100} value={activeLayer.opacity * 100} onChange={t681} className="w-full" /><span className="text-xs text-gray-400">{Math.round(activeLayer.opacity * 100)}%</span></div>;
  t233 = <><div className="px-3 py-2 border-b"><input value={searchQuery} onChange={t593} placeholder="Search layers..." className="w-full text-xs border rounded px-2 py-1" /></div><div className="flex-1 overflow-y-auto">{filteredLayers.map(t601)}</div><div className="px-3 py-2 border-t"><div className="flex justify-between items-center"><div className="text-xs text-gray-500">{stats.totalElements} elements · {stats.visibleLayers}/{layers.length} visible\n              </div><div className="flex gap-1"><button onClick={onLayerAdd} className="text-xs px-2 py-1 bg-blue-500 text-white rounded">\n                  +\n                </button>{t284}</div></div>{t308}</div></>;
  if ($[41] !== activeLayer || $[42] !== activeLayer || $[43] !== activeTab) {
    t347 = activeTab === "properties";
    $[41] = activeLayer;
    $[42] = activeLayer;
    $[43] = activeTab;
  }
  if ($[44] !== t385 || $[45] !== t399 || $[46] !== activeLayer || $[47] !== activeLayer || $[48] !== activeLayer || $[49] !== activeLayer) {
    $[44] = t385;
    $[45] = t399;
    $[46] = activeLayer;
    $[47] = activeLayer;
    $[48] = activeLayer;
    $[49] = activeLayer;
  }
  if (activeLayer) {
    if (activeLayer.visible) {
      t385 = "Yes";
    } else {
      t385 = "No";
    }
    if (activeLayer.locked) {
      t399 = "Yes";
    } else {
      t399 = "No";
    }
    t358 = <div className="space-y-2"><div><strong>Name:</strong>{activeLayer.name}</div><div><strong>Elements:</strong>{activeLayer.elements}</div><div><strong>Visible:</strong>{t385}</div><div><strong>Locked:</strong>{t399}</div><div><strong>Opacity:</strong>{Math.round(activeLayer.opacity * 100)}%</div></div>;
  } else {
    t358 = <p>Select a layer to view properties</p>;
  }
  t347 = <t827 className={t828}>{t358}</t827>;
  t427 = activeTab === "history";
  t427 = <div className="p-3 text-sm text-gray-500 text-center">\n          History panel (not implemented)\n        </div>;
  return <t565 className={t566}>{t583}{t233}{t347}{t427}</t565>;
}

