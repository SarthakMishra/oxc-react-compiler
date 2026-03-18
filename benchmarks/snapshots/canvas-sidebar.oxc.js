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
  const $ = _c(64);
  let t15;
  let t21;
  let t27;
  let t33;
  let t39;
  let t45;
  let t322;
  let t53;
  let t323;
  let t57;
  let t324;
  let t325;
  let t74;
  let t77;
  let t326;
  let t327;
  let t83;
  let t85;
  let t328;
  let t92;
  let t93;
  let t329;
  let commitEdit;
  let t330;
  let t109;
  let t111;
  let t331;
  let t332;
  let t117;
  let t118;
  let t333;
  let t334;
  let t335;
  let t133;
  let t134;
  let t336;
  let t165;
  let t207;
  let t226;
  let t258;
  let t265;
  let t288;
  let t298;
  let t319;
  let t145;
  let t146;
  let t147;
  let t164;
  let { layers, activeLayerId, onLayerSelect, onLayerToggleVisible, onLayerToggleLock, onLayerRename, onLayerReorder, onLayerDelete, onLayerAdd, onLayerDuplicate, onLayerOpacity } = t0;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t15 = "layers";
    $[0] = t15;
  } else {
    t15 = $[0];
  }
  let activeTab;
  let setActiveTab;
  ([activeTab, setActiveTab] = useState(t15));
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    t21 = null;
    $[1] = t21;
  } else {
    t21 = $[1];
  }
  let t22 = useState(t21);
  let editingId;
  let setEditingId;
  ([editingId, setEditingId] = t22);
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    $[2] = editingId;
    $[3] = setEditingId;
  } else {
    editingId = $[2];
    setEditingId = $[3];
  }
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    t27 = "";
    $[4] = t27;
  } else {
    t27 = $[4];
  }
  let editName;
  let setEditName;
  ([editName, setEditName] = useState(t27));
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    t33 = "";
    $[5] = t33;
  } else {
    t33 = $[5];
  }
  let searchQuery;
  let setSearchQuery;
  ([searchQuery, setSearchQuery] = useState(t33));
  if ($[6] === Symbol.for("react.memo_cache_sentinel")) {
    t39 = null;
    $[6] = t39;
  } else {
    t39 = $[6];
  }
  let t40 = useState(t39);
  let dragIndex;
  let setDragIndex;
  ([dragIndex, setDragIndex] = t40);
  if ($[7] === Symbol.for("react.memo_cache_sentinel")) {
    $[7] = dragIndex;
    $[8] = setDragIndex;
  } else {
    dragIndex = $[7];
    setDragIndex = $[8];
  }
  if ($[9] === Symbol.for("react.memo_cache_sentinel")) {
    t45 = false;
    $[9] = t45;
  } else {
    t45 = $[9];
  }
  let collapsed;
  let setCollapsed;
  ([collapsed, setCollapsed] = useState(t45));
  if ($[10] === Symbol.for("react.memo_cache_sentinel")) {
    t53 = null;
    $[10] = t322;
    $[11] = t53;
  } else {
    t322 = $[10];
    t53 = $[11];
  }
  let editInputRef = t322;
  let t54 = useRef(t53);
  if ($[12] === Symbol.for("react.memo_cache_sentinel")) {
    t323 = t54;
    $[12] = t323;
  } else {
    t323 = $[12];
  }
  editInputRef = t323;
  if ($[13] === Symbol.for("react.memo_cache_sentinel")) {
    t57 = () => {
      let t0;
      t0 = editingId;
      t0 = editInputRef.current;
      if (t0) {
        let t8 = editInputRef.current.focus();
        let t11 = editInputRef.current.select();
      }
      return undefined;
    };
    $[13] = t57;
  } else {
    t57 = $[13];
  }
  let t60 = useEffect(t57, [editingId]);
  let filteredLayers;
  let t65 = () => {
    if (!searchQuery) {
      return layers;
    }
    let q;
    q = searchQuery.toLowerCase();
    let t9 = (l) => {
      return l.name.toLowerCase().includes(q);
    };
    return layers.filter(t9);
  };
  let t69 = useMemo(t65, [layers, searchQuery]);
  if ($[14] !== t69 || $[15] !== activeLayerId || $[16] !== layers) {
    t324 = t69;
    t74 = () => {
      let t2 = (l) => {
        return l.id === activeLayerId;
      };
      return layers.find(t2);
    };
    t77 = [layers, activeLayerId];
    $[14] = t69;
    $[15] = activeLayerId;
    $[16] = layers;
    $[17] = t324;
    $[18] = t325;
    $[19] = t74;
    $[20] = t77;
  } else {
    t324 = $[17];
    t325 = $[18];
    t74 = $[19];
    t77 = $[20];
  }
  filteredLayers = t324;
  let activeLayer = t325;
  let t78 = useMemo(t74, t77);
  if ($[21] !== t78 || $[22] !== layers) {
    t326 = t78;
    t83 = () => {
      let t2 = (sum, l) => {
        return sum + l.elements;
      };
      let t6 = (l) => {
        return l.visible;
      };
      let t10 = (l) => {
        return l.locked;
      };
      return { totalElements: layers.reduce(t2, 0), visibleLayers: layers.filter(t6).length, lockedLayers: layers.filter(t10).length };
    };
    t85 = [layers];
    $[21] = t78;
    $[22] = layers;
    $[23] = t326;
    $[24] = t327;
    $[25] = t83;
    $[26] = t85;
  } else {
    t326 = $[23];
    t327 = $[24];
    t83 = $[25];
    t85 = $[26];
  }
  activeLayer = t326;
  let stats = t327;
  stats = useMemo(t83, t85);
  if ($[27] === Symbol.for("react.memo_cache_sentinel")) {
    t92 = (layer) => {
      let t5 = setEditingId(layer.id);
      let t10 = setEditName(layer.name);
      return undefined;
    };
    t93 = [];
    $[27] = t328;
    $[28] = t92;
    $[29] = t93;
  } else {
    t328 = $[27];
    t92 = $[28];
    t93 = $[29];
  }
  let startEditing = t328;
  let t94 = useCallback(t92, t93);
  if ($[30] !== t94) {
    t329 = t94;
    $[30] = t94;
    $[31] = t329;
  } else {
    t329 = $[31];
  }
  startEditing = t329;
  let t99 = () => {
    let t0;
    t0 = editingId;
    t0 = editName.trim();
    if (t0) {
      let t11 = onLayerRename(editingId, editName.trim());
    }
    let t15 = setEditingId(null);
    return undefined;
  };
  let t104 = useCallback(t99, [editingId, editName, onLayerRename]);
  if ($[32] !== t104) {
    commitEdit = t104;
    t109 = (e) => {
      if (e.key === "Enter") {
        let t7 = commitEdit();
      }
      if (e.key === "Escape") {
        let t15 = setEditingId(null);
      }
      return undefined;
    };
    t111 = [commitEdit];
    $[32] = t104;
    $[33] = commitEdit;
    $[34] = t330;
    $[35] = t109;
    $[36] = t111;
  } else {
    commitEdit = $[33];
    t330 = $[34];
    t109 = $[35];
    t111 = $[36];
  }
  let handleKeyDown = t330;
  let t112 = useCallback(t109, t111);
  if ($[37] !== t112) {
    t331 = t112;
    $[37] = t112;
    $[38] = t331;
  } else {
    t331 = $[38];
  }
  handleKeyDown = t331;
  if ($[39] === Symbol.for("react.memo_cache_sentinel")) {
    t117 = (index) => {
      let t4 = setDragIndex(index);
      return undefined;
    };
    t118 = [];
    $[39] = t332;
    $[40] = t117;
    $[41] = t118;
  } else {
    t332 = $[39];
    t117 = $[40];
    t118 = $[41];
  }
  let handleDragStart = t332;
  let t119 = useCallback(t117, t118);
  if ($[42] !== t119) {
    t333 = t119;
    $[42] = t119;
    $[43] = t333;
  } else {
    t333 = $[43];
  }
  handleDragStart = t333;
  let handleDragOver;
  let t124 = (e, index) => {
    let t3 = e.preventDefault();
    let t4;
    t4 = dragIndex !== null;
    t4 = dragIndex !== index;
    if (t4) {
      let t16 = onLayerReorder(dragIndex, index);
      let t20 = setDragIndex(index);
    }
    return undefined;
  };
  let t128 = useCallback(t124, [dragIndex, onLayerReorder]);
  if ($[44] !== t128) {
    t334 = t128;
    $[44] = t128;
    $[45] = t334;
  } else {
    t334 = $[45];
  }
  handleDragOver = t334;
  if ($[46] === Symbol.for("react.memo_cache_sentinel")) {
    t133 = () => {
      let t3 = setDragIndex(null);
      return undefined;
    };
    t134 = [];
    $[46] = t335;
    $[47] = t133;
    $[48] = t134;
  } else {
    t335 = $[46];
    t133 = $[47];
    t134 = $[48];
  }
  let handleDragEnd = t335;
  let t135 = useCallback(t133, t134);
  if ($[49] !== t135) {
    t336 = t135;
    if (collapsed) {
      if ($[50] === Symbol.for("react.memo_cache_sentinel")) {
        let t141 = () => {
          return setCollapsed(false);
        };
        t145 = <div className="w-10 bg-white border-l flex flex-col items-center py-2"><button onClick={t141} className="text-gray-500 hover:text-gray-700">\n          ◀\n        </button></div>;
        $[50] = t145;
      } else {
        t145 = $[50];
      }
      return t145;
    }
    if ($[51] === Symbol.for("react.memo_cache_sentinel")) {
      t146 = "div";
      t147 = "w-64 bg-white border-l flex flex-col h-full";
      let t156 = (tab) => {
        let t3 = () => {
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
      let t160 = () => {
        return setCollapsed(true);
      };
      t164 = (
        <div className="flex items-center justify-between px-3 py-2 border-b">
          <div className="flex gap-1">{["layers", "properties", "history"].map(t156)}</div>
          <button onClick={t160} className="text-gray-400 hover:text-gray-600">\n          ▶\n        </button>
        </div>
      );
      $[51] = t146;
      $[52] = t147;
      $[53] = t164;
      $[54] = t165;
    } else {
      t146 = $[51];
      t147 = $[52];
      t164 = $[53];
      t165 = $[54];
    }
    $[49] = t135;
    $[50] = t336;
    $[51] = t165;
    $[52] = t207;
    $[53] = t226;
    $[54] = t258;
    $[55] = t265;
    $[56] = t288;
    $[57] = t298;
    $[58] = t319;
  } else {
    t336 = $[50];
    t165 = $[51];
    t207 = $[52];
    t226 = $[53];
    t258 = $[54];
    t265 = $[55];
    t288 = $[56];
    t298 = $[57];
    t319 = $[58];
  }
  handleDragEnd = t336;
}

