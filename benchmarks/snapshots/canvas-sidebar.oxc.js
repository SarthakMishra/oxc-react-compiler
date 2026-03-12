import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
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
  const $ = _c(63);
  let layers;
  let activeLayerId;
  let onLayerSelect;
  let onLayerToggleVisible;
  let onLayerToggleLock;
  let onLayerRename;
  let onLayerReorder;
  let onLayerDelete;
  let onLayerAdd;
  let onLayerDuplicate;
  let onLayerOpacity;
  if ($[0] !== layers || $[1] !== activeLayerId || $[2] !== onLayerSelect || $[3] !== onLayerToggleVisible || $[4] !== onLayerToggleLock || $[5] !== onLayerRename || $[6] !== onLayerReorder || $[7] !== onLayerDelete || $[8] !== onLayerAdd || $[9] !== onLayerDuplicate || $[10] !== onLayerOpacity) {
    $[0] = layers;
    $[1] = activeLayerId;
    $[2] = onLayerSelect;
    $[3] = onLayerToggleVisible;
    $[4] = onLayerToggleLock;
    $[5] = onLayerRename;
    $[6] = onLayerReorder;
    $[7] = onLayerDelete;
    $[8] = onLayerAdd;
    $[9] = onLayerDuplicate;
    $[10] = onLayerOpacity;
  } else {
  }
  ({ layers, activeLayerId, onLayerSelect, onLayerToggleVisible, onLayerToggleLock, onLayerRename, onLayerReorder, onLayerDelete, onLayerAdd, onLayerDuplicate, onLayerOpacity } = t0);
  const t429 = useState;
  const t430 = "layers";
  const t431 = t429(t430);
  let activeTab;
  let setActiveTab;
  if ($[11] !== activeTab || $[12] !== setActiveTab) {
    $[11] = activeTab;
    $[12] = setActiveTab;
  } else {
  }
  ([activeTab, setActiveTab] = t431);
  const t435 = useState;
  const t436 = null;
  const t437 = t435(t436);
  let editingId;
  let setEditingId;
  if ($[13] !== editingId || $[14] !== setEditingId) {
    $[13] = editingId;
    $[14] = setEditingId;
  } else {
  }
  ([editingId, setEditingId] = t437);
  const t441 = useState;
  const t442 = "";
  const t443 = t441(t442);
  let editName;
  let setEditName;
  if ($[15] !== editName || $[16] !== setEditName) {
    $[15] = editName;
    $[16] = setEditName;
  } else {
  }
  ([editName, setEditName] = t443);
  const t447 = useState;
  const t448 = "";
  const t449 = t447(t448);
  let searchQuery;
  let setSearchQuery;
  if ($[17] !== searchQuery || $[18] !== setSearchQuery) {
    $[17] = searchQuery;
    $[18] = setSearchQuery;
  } else {
  }
  ([searchQuery, setSearchQuery] = t449);
  const t453 = useState;
  const t454 = null;
  const t455 = t453(t454);
  let dragIndex;
  let setDragIndex;
  if ($[19] !== dragIndex || $[20] !== setDragIndex) {
    $[19] = dragIndex;
    $[20] = setDragIndex;
  } else {
  }
  ([dragIndex, setDragIndex] = t455);
  const t459 = useState;
  const t460 = false;
  const t461 = t459(t460);
  let collapsed;
  let setCollapsed;
  if ($[21] !== collapsed || $[22] !== setCollapsed) {
    $[21] = collapsed;
    $[22] = setCollapsed;
  } else {
  }
  ([collapsed, setCollapsed] = t461);
  let editInputRef;
  if ($[23] !== editInputRef) {
    $[23] = editInputRef;
  } else {
  }
  const t466 = useRef;
  const t467 = null;
  const t468 = t466(t467);
  editInputRef = t468;
  const t470 = useEffect;
  const t471 = () => {
    const t1 = editingId;
    const t3 = editInputRef;
    const t4 = t3.current;
    const t7 = editInputRef;
    const t8 = t7.current;
    const t9 = t8.focus();
    const t11 = editInputRef;
    const t12 = t11.current;
    const t13 = t12.select();
    const t14 = undefined;
    return t14;
  };
  let filteredLayers;
  if ($[24] !== editingId || $[25] !== t470 || $[26] !== t471 || $[27] !== filteredLayers) {
    const t472 = editingId;
    const t473 = [t472];
    const t474 = t470(t471, t473);
    $[24] = editingId;
    $[25] = t470;
    $[26] = t471;
    $[27] = filteredLayers;
  } else {
  }
  let activeLayer;
  if ($[28] !== useMemo || $[29] !== layers || $[30] !== searchQuery || $[31] !== filteredLayers || $[32] !== activeLayer) {
    const t476 = useMemo;
    const t477 = () => {
      const t1 = searchQuery;
      const t2 = !t1;
      const t4 = layers;
      return t4;
      let q;
      const t8 = searchQuery;
      const t9 = t8.toLowerCase();
      q = t9;
      const t12 = layers;
      const t13 = (l) => {
        const t2 = l;
        const t3 = t2.name;
        const t4 = t3.toLowerCase();
        const t6 = q;
        const t7 = t4.includes(t6);
        return t7;
      };
      const t14 = t12.filter(t13);
      return t14;
      const t15 = undefined;
      return t15;
    };
    const t478 = layers;
    const t479 = searchQuery;
    const t480 = [t478, t479];
    const t481 = t476(t477, t480);
    filteredLayers = t481;
    $[28] = useMemo;
    $[29] = layers;
    $[30] = searchQuery;
    $[31] = filteredLayers;
    $[32] = activeLayer;
  } else {
  }
  let stats;
  if ($[33] !== useMemo || $[34] !== layers || $[35] !== activeLayerId || $[36] !== activeLayer || $[37] !== stats) {
    const t484 = useMemo;
    const t485 = () => {
      const t1 = layers;
      const t2 = (l) => {
        const t2 = l;
        const t3 = t2.id;
        const t5 = activeLayerId;
        const t6 = t3 === t5;
        return t6;
      };
      const t3 = t1.find(t2);
      return t3;
    };
    const t486 = layers;
    const t487 = activeLayerId;
    const t488 = [t486, t487];
    const t489 = t484(t485, t488);
    activeLayer = t489;
    $[33] = useMemo;
    $[34] = layers;
    $[35] = activeLayerId;
    $[36] = activeLayer;
    $[37] = stats;
  } else {
  }
  let startEditing;
  if ($[38] !== useMemo || $[39] !== layers || $[40] !== stats || $[41] !== startEditing) {
    const t492 = useMemo;
    const t493 = () => {
      const t1 = layers;
      const t2 = (sum, l) => {
        const t3 = sum;
        const t5 = l;
        const t6 = t5.elements;
        const t7 = t3 + t6;
        return t7;
      };
      const t3 = 0;
      const t4 = t1.reduce(t2, t3);
      const t6 = layers;
      const t7 = (l) => {
        const t2 = l;
        const t3 = t2.visible;
        return t3;
      };
      const t8 = t6.filter(t7);
      const t9 = t8.length;
      const t11 = layers;
      const t12 = (l) => {
        const t2 = l;
        const t3 = t2.locked;
        return t3;
      };
      const t13 = t11.filter(t12);
      const t14 = t13.length;
      const t15 = { totalElements: t4, visibleLayers: t9, lockedLayers: t14 };
      return t15;
    };
    const t494 = layers;
    const t495 = [t494];
    const t496 = t492(t493, t495);
    stats = t496;
    $[38] = useMemo;
    $[39] = layers;
    $[40] = stats;
    $[41] = startEditing;
  } else {
  }
  const t499 = useCallback;
  const t500 = (layer) => {
    const t2 = setEditingId;
    const t4 = layer;
    const t5 = t4.id;
    const t6 = t2(t5);
    const t8 = setEditName;
    const t10 = layer;
    const t11 = t10.name;
    const t12 = t8(t11);
    const t13 = undefined;
    return t13;
  };
  const t501 = [];
  const t502 = t499(t500, t501);
  startEditing = t502;
  let commitEdit;
  if ($[42] !== commitEdit) {
    $[42] = commitEdit;
  } else {
  }
  let handleKeyDown;
  if ($[43] !== useCallback || $[44] !== editingId || $[45] !== editName || $[46] !== onLayerRename || $[47] !== commitEdit || $[48] !== handleKeyDown) {
    const t505 = useCallback;
    const t506 = () => {
      const t1 = editingId;
      const t3 = editName;
      const t4 = t3.trim();
      const t7 = onLayerRename;
      const t9 = editingId;
      const t11 = editName;
      const t12 = t11.trim();
      const t13 = t7(t9, t12);
      const t15 = setEditingId;
      const t16 = null;
      const t17 = t15(t16);
      const t18 = undefined;
      return t18;
    };
    const t507 = editingId;
    const t508 = editName;
    const t509 = onLayerRename;
    const t510 = [t507, t508, t509];
    const t511 = t505(t506, t510);
    commitEdit = t511;
    $[43] = useCallback;
    $[44] = editingId;
    $[45] = editName;
    $[46] = onLayerRename;
    $[47] = commitEdit;
    $[48] = handleKeyDown;
  } else {
  }
  let handleDragStart;
  if ($[49] !== useCallback || $[50] !== commitEdit || $[51] !== handleKeyDown || $[52] !== handleDragStart) {
    const t514 = useCallback;
    const t515 = (e) => {
      const t2 = e;
      const t3 = t2.key;
      const t4 = "Enter";
      const t5 = t3 === t4;
      const t7 = commitEdit;
      const t8 = t7();
      const t10 = e;
      const t11 = t10.key;
      const t12 = "Escape";
      const t13 = t11 === t12;
      const t15 = setEditingId;
      const t16 = null;
      const t17 = t15(t16);
      const t18 = undefined;
      return t18;
    };
    const t516 = commitEdit;
    const t517 = [t516];
    const t518 = t514(t515, t517);
    handleKeyDown = t518;
    $[49] = useCallback;
    $[50] = commitEdit;
    $[51] = handleKeyDown;
    $[52] = handleDragStart;
  } else {
  }
  const t521 = useCallback;
  const t522 = (index) => {
    const t2 = setDragIndex;
    const t4 = index;
    const t5 = t2(t4);
    const t6 = undefined;
    return t6;
  };
  const t523 = [];
  const t524 = t521(t522, t523);
  handleDragStart = t524;
  let handleDragOver;
  if ($[53] !== handleDragOver) {
    $[53] = handleDragOver;
  } else {
  }
  let handleDragEnd;
  if ($[54] !== useCallback || $[55] !== dragIndex || $[56] !== onLayerReorder || $[57] !== handleDragOver || $[58] !== handleDragEnd) {
    const t527 = useCallback;
    const t528 = (e, index) => {
      const t3 = e;
      const t4 = t3.preventDefault();
      const t6 = dragIndex;
      const t7 = null;
      const t8 = t6 !== t7;
      const t10 = dragIndex;
      const t12 = index;
      const t13 = t10 !== t12;
      const t16 = onLayerReorder;
      const t18 = dragIndex;
      const t20 = index;
      const t21 = t16(t18, t20);
      const t23 = setDragIndex;
      const t25 = index;
      const t26 = t23(t25);
      const t27 = undefined;
      return t27;
    };
    const t529 = dragIndex;
    const t530 = onLayerReorder;
    const t531 = [t529, t530];
    const t532 = t527(t528, t531);
    handleDragOver = t532;
    $[54] = useCallback;
    $[55] = dragIndex;
    $[56] = onLayerReorder;
    $[57] = handleDragOver;
    $[58] = handleDragEnd;
  } else {
  }
  const t535 = useCallback;
  const t536 = () => {
    const t1 = setDragIndex;
    const t2 = null;
    const t3 = t1(t2);
    const t4 = undefined;
    return t4;
  };
  const t537 = [];
  const t538 = t535(t536, t537);
  handleDragEnd = t538;
  let t540;
  if ($[59] !== collapsed) {
    t540 = collapsed;
    $[60] = t540;
    $[59] = collapsed;
  } else {
    t540 = $[60];
  }
  if (t540) {
    const t957 = "div";
    const t958 = "w-10 bg-white border-l flex flex-col items-center py-2";
    const t959 = "button";
    const t960 = () => {
      const t1 = setCollapsed;
      const t2 = false;
      const t3 = t1(t2);
      return t3;
    };
    const t961 = "text-gray-500 hover:text-gray-700";
    const t962 = "\n          ◀\n        ";
    const t963 = _jsx(t959, { onClick: t960, className: t961, children: t962 });
    const t964 = _jsx(t957, { className: t958, children: t963 });
    return t964;
  } else {
    const t541 = "div";
    const t542 = "w-64 bg-white border-l flex flex-col h-full";
    const t543 = "div";
    const t544 = "flex items-center justify-between px-3 py-2 border-b";
    const t545 = "div";
    const t546 = "flex gap-1";
    const t547 = "layers";
    const t548 = "properties";
    const t549 = "history";
    const t550 = [t547, t548, t549];
    const t551 = (tab) => {
      const t1 = "button";
      const t3 = tab;
      const t4 = () => {
        const t1 = setActiveTab;
        const t3 = tab;
        const t4 = t1(t3);
        return t4;
      };
      const t6 = activeTab;
      const t8 = tab;
      const t9 = t6 === t8;
      const t10 = "bg-blue-100 text-blue-700";
      const t11 = "text-gray-500 hover:bg-gray-100";
      const t13 = `px-2 py-1 text-xs rounded ${t12}`;
      const t15 = tab;
      const t16 = 0;
      const t17 = t15.charAt(t16);
      const t18 = t17.toUpperCase();
      const t20 = tab;
      const t21 = 1;
      const t22 = t20.slice(t21);
      const t23 = t18 + t22;
      const t24 = _jsx(t1, { key: t3, onClick: t4, className: t13, children: t23 });
      return t24;
    };
    const t552 = t550.map(t551);
    const t553 = _jsx(t545, { className: t546, children: t552 });
    const t554 = "button";
    const t555 = () => {
      const t1 = setCollapsed;
      const t2 = true;
      const t3 = t1(t2);
      return t3;
    };
    const t556 = "text-gray-400 hover:text-gray-600";
    const t557 = "\n          ▶\n        ";
    const t558 = _jsx(t554, { onClick: t555, className: t556, children: t557 });
    const t559 = _jsxs(t543, { className: t544, children: [t553, t558] });
    if ($[61] !== activeTab) {
      const t560 = activeTab;
      $[61] = activeTab;
    } else {
    }
    const t561 = "layers";
  }
  const t541 = "div";
  const t542 = "w-64 bg-white border-l flex flex-col h-full";
  const t543 = "div";
  const t544 = "flex items-center justify-between px-3 py-2 border-b";
  const t545 = "div";
  const t546 = "flex gap-1";
  const t547 = "layers";
  const t548 = "properties";
  const t549 = "history";
  const t550 = [t547, t548, t549];
  const t551 = (tab) => {
    const t1 = "button";
    const t3 = tab;
    const t4 = () => {
      const t1 = setActiveTab;
      const t3 = tab;
      const t4 = t1(t3);
      return t4;
    };
    const t6 = activeTab;
    const t8 = tab;
    const t9 = t6 === t8;
    const t10 = "bg-blue-100 text-blue-700";
    const t11 = "text-gray-500 hover:bg-gray-100";
    const t13 = `px-2 py-1 text-xs rounded ${t12}`;
    const t15 = tab;
    const t16 = 0;
    const t17 = t15.charAt(t16);
    const t18 = t17.toUpperCase();
    const t20 = tab;
    const t21 = 1;
    const t22 = t20.slice(t21);
    const t23 = t18 + t22;
    const t24 = _jsx(t1, { key: t3, onClick: t4, className: t13, children: t23 });
    return t24;
  };
  const t552 = t550.map(t551);
  const t553 = _jsx(t545, { className: t546, children: t552 });
  const t554 = "button";
  const t555 = () => {
    const t1 = setCollapsed;
    const t2 = true;
    const t3 = t1(t2);
    return t3;
  };
  const t556 = "text-gray-400 hover:text-gray-600";
  const t557 = "\n          ▶\n        ";
  const t558 = _jsx(t554, { onClick: t555, className: t556, children: t557 });
  const t559 = _jsxs(t543, { className: t544, children: [t553, t558] });
  if ($[62] !== activeTab) {
    const t560 = activeTab;
    $[62] = activeTab;
  } else {
  }
  const t561 = "layers";
}

