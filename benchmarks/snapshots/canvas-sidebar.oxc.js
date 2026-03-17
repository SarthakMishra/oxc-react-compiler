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
  const $ = _c(74);
  const { layers, activeLayerId, onLayerSelect, onLayerToggleVisible, onLayerToggleLock, onLayerRename, onLayerReorder, onLayerDelete, onLayerAdd, onLayerDuplicate, onLayerOpacity } = t0;
  let t15;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    $[0] = t15;
  } else {
    t15 = $[0];
  }
  let t328;
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    $[1] = t328;
  } else {
    t328 = $[1];
  }
  const editingId = t328;
  let setEditingId;
  let t27;
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    $[2] = t27;
  } else {
    t27 = $[2];
  }
  let t33;
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    $[3] = t33;
  } else {
    t33 = $[3];
  }
  let t329;
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    $[4] = t329;
  } else {
    t329 = $[4];
  }
  const dragIndex = t329;
  let setDragIndex;
  let t45;
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    $[5] = t45;
  } else {
    t45 = $[5];
  }
  let t330;
  let t53;
  if ($[6] === Symbol.for("react.memo_cache_sentinel")) {
    t53 = null;
    $[6] = t330;
    $[7] = t53;
  } else {
    t330 = $[6];
    t53 = $[7];
  }
  const editInputRef = t330;
  const t54 = useRef(t53);
  let t331;
  if ($[8] === Symbol.for("react.memo_cache_sentinel")) {
    t331 = t54;
    $[8] = t331;
  } else {
    t331 = $[8];
  }
  const editInputRef = t331;
  let t57;
  if ($[9] === Symbol.for("react.memo_cache_sentinel")) {
    t57 = () => {
      let t0;
      t0 = editingId;
      t0 = editInputRef.current;
      if (t0) {
        const t8 = editInputRef.current.focus();
        const t11 = editInputRef.current.select();
      }
      return undefined;
    };
    $[9] = t57;
  } else {
    t57 = $[9];
  }
  let t332;
  let t99;
  let t103;
  let t21;
  let t59;
  if ($[10] !== onLayerRename) {
    t59 = [editingId];
    $[10] = onLayerRename;
    $[11] = t332;
    $[12] = t99;
    $[13] = t103;
    $[14] = t21;
    $[15] = t59;
  } else {
    t332 = $[11];
    t99 = $[12];
    t103 = $[13];
    t21 = $[14];
    t59 = $[15];
  }
  const commitEdit = t332;
  const t60 = useEffect(t57, t59);
  let filteredLayers;
  const t65 = () => {
    if (!searchQuery) {
      return layers;
    }
    let q;
    q = searchQuery.toLowerCase();
    const t9 = (l) => {
      return l.name.toLowerCase().includes(q);
    };
    return layers.filter(t9);
  };
  const t69 = useMemo(t65, [layers, searchQuery]);
  let t334;
  let t333;
  let t74;
  let t77;
  if ($[16] !== t69 || $[17] !== activeLayerId || $[18] !== layers) {
    t333 = t69;
    t74 = () => {
      const t2 = (l) => {
        return l.id === activeLayerId;
      };
      return layers.find(t2);
    };
    t77 = [layers, activeLayerId];
    $[16] = t69;
    $[17] = activeLayerId;
    $[18] = layers;
    $[19] = t333;
    $[20] = t334;
    $[21] = t74;
    $[22] = t77;
  } else {
    t333 = $[19];
    t334 = $[20];
    t74 = $[21];
    t77 = $[22];
  }
  filteredLayers = t333;
  const activeLayer = t334;
  const t78 = useMemo(t74, t77);
  let t336;
  let t335;
  let t83;
  let t85;
  if ($[23] !== t78 || $[24] !== layers) {
    t335 = t78;
    t83 = () => {
      const t2 = (sum, l) => {
        return sum + l.elements;
      };
      const t6 = (l) => {
        return l.visible;
      };
      const t10 = (l) => {
        return l.locked;
      };
      return { totalElements: layers.reduce(t2, 0), visibleLayers: layers.filter(t6).length, lockedLayers: layers.filter(t10).length };
    };
    t85 = [layers];
    $[23] = t78;
    $[24] = layers;
    $[25] = t335;
    $[26] = t336;
    $[27] = t83;
    $[28] = t85;
  } else {
    t335 = $[25];
    t336 = $[26];
    t83 = $[27];
    t85 = $[28];
  }
  const activeLayer = t335;
  const stats = t336;
  const t86 = useMemo(t83, t85);
  let t337;
  if ($[29] !== t86) {
    t337 = t86;
    $[29] = t86;
    $[30] = t337;
  } else {
    t337 = $[30];
  }
  const stats = t337;
  let t338;
  let t92;
  let t93;
  if ($[31] === Symbol.for("react.memo_cache_sentinel")) {
    t92 = (layer) => {
      const t5 = setEditingId(layer.id);
      const t10 = setEditName(layer.name);
      return undefined;
    };
    t93 = [];
    $[31] = t338;
    $[32] = t92;
    $[33] = t93;
  } else {
    t338 = $[31];
    t92 = $[32];
    t93 = $[33];
  }
  const startEditing = t338;
  const t94 = useCallback(t92, t93);
  let t339;
  if ($[34] !== t94) {
    t339 = t94;
    $[34] = t94;
    $[35] = t339;
  } else {
    t339 = $[35];
  }
  const startEditing = t339;
  let commitEdit;
  t99 = () => {
    let t0;
    t0 = editingId;
    t0 = editName.trim();
    if (t0) {
      const t11 = onLayerRename(editingId, editName.trim());
    }
    const t15 = setEditingId(null);
    return undefined;
  };
  const t104 = useCallback(t99, [editingId, editName, onLayerRename]);
  let t340;
  let t109;
  let t111;
  if ($[36] !== t104) {
    commitEdit = t104;
    t109 = (e) => {
      if (e.key === "Enter") {
        const t7 = commitEdit();
      }
      if (e.key === "Escape") {
        const t15 = setEditingId(null);
      }
      return undefined;
    };
    t111 = [commitEdit];
    $[36] = t104;
    $[37] = commitEdit;
    $[38] = t340;
    $[39] = t109;
    $[40] = t111;
  } else {
    commitEdit = $[37];
    t340 = $[38];
    t109 = $[39];
    t111 = $[40];
  }
  const handleKeyDown = t340;
  const t112 = useCallback(t109, t111);
  let t341;
  if ($[41] !== t112) {
    t341 = t112;
    $[41] = t112;
    $[42] = t341;
  } else {
    t341 = $[42];
  }
  const handleKeyDown = t341;
  let t342;
  let t117;
  let t118;
  if ($[43] === Symbol.for("react.memo_cache_sentinel")) {
    t117 = (index) => {
      const t4 = setDragIndex(index);
      return undefined;
    };
    t118 = [];
    $[43] = t342;
    $[44] = t117;
    $[45] = t118;
  } else {
    t342 = $[43];
    t117 = $[44];
    t118 = $[45];
  }
  const handleDragStart = t342;
  const t119 = useCallback(t117, t118);
  let t343;
  if ($[46] !== t119) {
    t343 = t119;
    $[46] = t119;
    $[47] = t343;
  } else {
    t343 = $[47];
  }
  const handleDragStart = t343;
  let t344;
  let t39;
  let t124;
  let t127;
  if ($[48] !== onLayerReorder) {
    t124 = (e, index) => {
      const t3 = e.preventDefault();
      let t4;
      t4 = dragIndex !== null;
      t4 = dragIndex !== index;
      if (t4) {
        const t16 = onLayerReorder(dragIndex, index);
        const t20 = setDragIndex(index);
      }
      return undefined;
    };
    t127 = [dragIndex, onLayerReorder];
    $[48] = onLayerReorder;
    $[49] = t39;
    $[50] = t344;
    $[51] = t124;
    $[52] = t127;
  } else {
    t39 = $[49];
    t344 = $[50];
    t124 = $[51];
    t127 = $[52];
  }
  const handleDragOver = t344;
  const t128 = useCallback(t124, t127);
  let t345;
  if ($[53] !== t128) {
    t345 = t128;
    $[53] = t128;
    $[54] = t345;
  } else {
    t345 = $[54];
  }
  const handleDragOver = t345;
  let handleDragEnd;
  const t133 = () => {
    const t3 = setDragIndex(null);
    return undefined;
  };
  const t135 = useCallback(t133, []);
  let t346;
  if ($[55] !== t135) {
    t346 = t135;
    $[55] = t135;
    $[56] = t346;
  } else {
    t346 = $[56];
  }
  handleDragEnd = t346;
  if (collapsed) {
    let t145;
    if ($[57] === Symbol.for("react.memo_cache_sentinel")) {
      const t141 = () => {
        return setCollapsed(false);
      };
      t145 = <div className="w-10 bg-white border-l flex flex-col items-center py-2"><button onClick={t141} className="text-gray-500 hover:text-gray-700">\n          ◀\n        </button></div>;
      $[57] = t145;
    } else {
      t145 = $[57];
    }
    return t145;
  }
  let t327;
  let t347;
  let t133;
  let t134;
  let t348;
  let t65;
  let t68;
  if ($[58] !== t69 || $[59] !== activeLayer.opacity || $[60] !== activeLayer.name || $[61] !== activeLayer.elements || $[62] !== activeLayerId || $[63] !== layers || $[64] !== onLayerAdd || $[65] !== stats.totalElements || $[66] !== stats.visibleLayers) {
    const t156 = (tab) => {
      const t3 = () => {
        return setActiveTab(tab);
      };
      let t8;
      if (activeTab === tab) {
        t8 = "bg-blue-100 text-blue-700";
      } else {
        t8 = "text-gray-500 hover:bg-gray-100";
      }
      return <button key={tab} onClick={t3} className={`px-2 py-1 text-xs rounded ${t8}`}>{tab.charAt(0).toUpperCase() + tab.slice(1)}</button>;
    };
    const t160 = () => {
      return setCollapsed(true);
    };
    $[58] = t69;
    $[59] = activeLayer.opacity;
    $[60] = activeLayer.name;
    $[61] = activeLayer.elements;
    $[62] = activeLayerId;
    $[63] = layers;
    $[64] = onLayerAdd;
    $[65] = stats.totalElements;
    $[66] = stats.visibleLayers;
    $[67] = t327;
    $[68] = t347;
    $[69] = t133;
    $[70] = t134;
    $[71] = t348;
    $[72] = t65;
    $[73] = t68;
  } else {
    t327 = $[67];
    t347 = $[68];
    t133 = $[69];
    t134 = $[70];
    t348 = $[71];
    t65 = $[72];
    t68 = $[73];
  }
  handleDragEnd = t347;
  filteredLayers = t348;
  t165 = activeTab === "layers";
  const t173 = (e) => {
    return setSearchQuery(e.target.value);
  };
  const t181 = (layer, index) => {
    const t6 = () => {
      return handleDragStart(index);
    };
    const t7 = (e) => {
      return handleDragOver(e, index);
    };
    const t10 = () => {
      return onLayerSelect(layer.id);
    };
    let t16;
    if (layer.id === activeLayerId) {
      t16 = "bg-blue-50";
    } else {
      t16 = "hover:bg-gray-50";
    }
    let t23;
    if (dragIndex === index) {
      t23 = "opacity-50";
    } else {
      t23 = "";
    }
    const t28 = (e) => {
      const t2 = e.stopPropagation();
      const t8 = onLayerToggleVisible(layer.id);
      return undefined;
    };
    let t32;
    if (layer.visible) {
      t32 = "👁";
    } else {
      t32 = "○";
    }
    let t41;
    if (editingId === layer.id) {
      const t47 = (e) => {
        return setEditName(e.target.value);
      };
      t41 = <input ref={editInputRef} value={editName} onChange={t47} onBlur={commitEdit} onKeyDown={handleKeyDown} className="flex-1 text-xs border rounded px-1" />;
    } else {
      const t55 = () => {
        return startEditing(layer);
      };
      let t59;
      if (!layer.visible) {
        t59 = "text-gray-400";
      } else {
        t59 = "";
      }
      t41 = <span onDoubleClick={t55} className={`flex-1 truncate ${t59}`}>{layer.name}</span>;
    }
    const t72 = (e) => {
      const t2 = e.stopPropagation();
      const t8 = onLayerToggleLock(layer.id);
      return undefined;
    };
    let t76;
    if (layer.locked) {
      t76 = "🔒";
    } else {
      t76 = "🔓";
    }
    return <div key={layer.id} draggable onDragStart={t6} onDragOver={t7} onDragEnd={handleDragEnd} onClick={t10} className={`flex items-center px-3 py-2 text-sm cursor-pointer border-b ${t16} ${t23}`}><button onClick={t28} className="mr-2 text-xs">{t32}</button>{t41}<span className="text-xs text-gray-400 ml-1">{layer.elements}</span><button onClick={t72} className="ml-1 text-xs">{t76}</button></div>;
  };
  let t207;
  t207 = activeLayer;
  const t210 = () => {
    return onLayerDuplicate(activeLayerId);
  };
  const t215 = () => {
    return onLayerDelete(activeLayerId);
  };
  t207 = <><button onClick={t210} className="text-xs px-2 py-1 border rounded">\n                      ⧉\n                    </button><button onClick={t215} className="text-xs px-2 py-1 border rounded text-red-500" disabled={layers.length <= 1}>\n                      🗑\n                    </button></>;
  let t226;
  t226 = activeLayer;
  const t242 = (e) => {
    return onLayerOpacity(activeLayerId, parseInt(e.target.value) / 100);
  };
  t226 = <div className="mt-2"><label className="text-xs text-gray-500">Opacity</label><input type="range" min={0} max={100} value={activeLayer.opacity * 100} onChange={t242} className="w-full" /><span className="text-xs text-gray-400">{Math.round(activeLayer.opacity * 100)}%</span></div>;
  t165 = <><div className="px-3 py-2 border-b"><input value={searchQuery} onChange={t173} placeholder="Search layers..." className="w-full text-xs border rounded px-2 py-1" /></div><div className="flex-1 overflow-y-auto">{filteredLayers.map(t181)}</div><div className="px-3 py-2 border-t"><div className="flex justify-between items-center"><div className="text-xs text-gray-500">{stats.totalElements} elements · {stats.visibleLayers}/{layers.length} visible\n              </div><div className="flex gap-1"><button onClick={onLayerAdd} className="text-xs px-2 py-1 bg-blue-500 text-white rounded">\n                  +\n                </button>{t207}</div></div>{t226}</div></>;
  let t258;
  t258 = activeTab === "properties";
  let t265;
  if (activeLayer) {
    let t288;
    if (activeLayer.visible) {
      t288 = "Yes";
    } else {
      t288 = "No";
    }
    let t298;
    if (activeLayer.locked) {
      t298 = "Yes";
    } else {
      t298 = "No";
    }
    t265 = <div className="space-y-2"><div><strong>Name:</strong>{activeLayer.name}</div><div><strong>Elements:</strong>{activeLayer.elements}</div><div><strong>Visible:</strong>{t288}</div><div><strong>Locked:</strong>{t298}</div><div><strong>Opacity:</strong>{Math.round(activeLayer.opacity * 100)}%</div></div>;
  } else {
    t265 = <p>Select a layer to view properties</p>;
  }
  t258 = <div className="p-3 text-sm text-gray-500">{t265}</div>;
  let t319;
  t319 = activeTab === "history";
  t319 = <div className="p-3 text-sm text-gray-500 text-center">\n          History panel (not implemented)\n        </div>;
  return <t146 className={t147}>{t164}{t165}{t258}{t319}</t146>;
}

