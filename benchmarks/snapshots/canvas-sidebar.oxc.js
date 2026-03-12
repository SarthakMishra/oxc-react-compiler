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
  const $ = _c(89);
  const { layers, activeLayerId, onLayerSelect, onLayerToggleVisible, onLayerToggleLock, onLayerRename, onLayerReorder, onLayerDelete, onLayerAdd, onLayerDuplicate, onLayerOpacity } = t0;
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
  const t453 = useState;
  const t454 = "layers";
  const t455 = t453(t454);
  let activeTab;
  let setActiveTab;
  if ($[11] !== activeTab || $[12] !== setActiveTab) {
    $[11] = activeTab;
    $[12] = setActiveTab;
  } else {
  }
  ([activeTab, setActiveTab] = t455);
  const t459 = useState;
  const t460 = null;
  const t461 = t459(t460);
  let editingId;
  let setEditingId;
  if ($[13] !== editingId || $[14] !== setEditingId) {
    $[13] = editingId;
    $[14] = setEditingId;
  } else {
  }
  ([editingId, setEditingId] = t461);
  const t465 = useState;
  const t466 = "";
  const t467 = t465(t466);
  let editName;
  let setEditName;
  if ($[15] !== editName || $[16] !== setEditName) {
    $[15] = editName;
    $[16] = setEditName;
  } else {
  }
  ([editName, setEditName] = t467);
  const t471 = useState;
  const t472 = "";
  const t473 = t471(t472);
  let searchQuery;
  let setSearchQuery;
  if ($[17] !== searchQuery || $[18] !== setSearchQuery) {
    $[17] = searchQuery;
    $[18] = setSearchQuery;
  } else {
  }
  ([searchQuery, setSearchQuery] = t473);
  const t477 = useState;
  const t478 = null;
  const t479 = t477(t478);
  let dragIndex;
  let setDragIndex;
  if ($[19] !== dragIndex || $[20] !== setDragIndex) {
    $[19] = dragIndex;
    $[20] = setDragIndex;
  } else {
  }
  ([dragIndex, setDragIndex] = t479);
  const t483 = useState;
  const t484 = false;
  const t485 = t483(t484);
  let collapsed;
  let setCollapsed;
  if ($[21] !== collapsed || $[22] !== setCollapsed) {
    $[21] = collapsed;
    $[22] = setCollapsed;
  } else {
  }
  ([collapsed, setCollapsed] = t485);
  let editInputRef;
  if ($[23] !== editInputRef) {
    $[23] = editInputRef;
  } else {
  }
  const t490 = useRef;
  const t491 = null;
  const t492 = t490(t491);
  editInputRef = t492;
  const t494 = useEffect;
  const t495 = () => {
    let t0;
    const t3 = editingId;
    t0 = t3;
    const t6 = editInputRef;
    const t7 = t6.current;
    t0 = t7;
    if (t0) {
      const t10 = editInputRef;
      const t11 = t10.current;
      const t12 = t11.focus();
      const t14 = editInputRef;
      const t15 = t14.current;
      const t16 = t15.select();
    } else {
    }
    const t17 = undefined;
    return t17;
  };
  let filteredLayers;
  if ($[24] !== editingId || $[25] !== t494 || $[26] !== t495 || $[27] !== filteredLayers) {
    const t496 = editingId;
    const t497 = [t496];
    const t498 = t494(t495, t497);
    $[24] = editingId;
    $[25] = t494;
    $[26] = t495;
    $[27] = filteredLayers;
  } else {
  }
  let activeLayer;
  if ($[28] !== useMemo || $[29] !== layers || $[30] !== searchQuery || $[31] !== filteredLayers || $[32] !== activeLayer) {
    const t500 = useMemo;
    const t501 = () => {
      const t1 = searchQuery;
      const t2 = !t1;
      if (t2) {
        const t4 = layers;
        return t4;
      } else {
      }
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
    };
    const t502 = layers;
    const t503 = searchQuery;
    const t504 = [t502, t503];
    const t505 = t500(t501, t504);
    filteredLayers = t505;
    $[28] = useMemo;
    $[29] = layers;
    $[30] = searchQuery;
    $[31] = filteredLayers;
    $[32] = activeLayer;
  } else {
  }
  let stats;
  if ($[33] !== useMemo || $[34] !== layers || $[35] !== activeLayerId || $[36] !== activeLayer || $[37] !== stats) {
    const t508 = useMemo;
    const t509 = () => {
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
    const t510 = layers;
    const t511 = activeLayerId;
    const t512 = [t510, t511];
    const t513 = t508(t509, t512);
    activeLayer = t513;
    $[33] = useMemo;
    $[34] = layers;
    $[35] = activeLayerId;
    $[36] = activeLayer;
    $[37] = stats;
  } else {
  }
  let startEditing;
  if ($[38] !== useMemo || $[39] !== layers || $[40] !== stats || $[41] !== startEditing) {
    const t516 = useMemo;
    const t517 = () => {
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
    const t518 = layers;
    const t519 = [t518];
    const t520 = t516(t517, t519);
    stats = t520;
    $[38] = useMemo;
    $[39] = layers;
    $[40] = stats;
    $[41] = startEditing;
  } else {
  }
  const t523 = useCallback;
  const t524 = (layer) => {
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
  const t525 = [];
  const t526 = t523(t524, t525);
  startEditing = t526;
  let commitEdit;
  if ($[42] !== commitEdit) {
    $[42] = commitEdit;
  } else {
  }
  let handleKeyDown;
  if ($[43] !== useCallback || $[44] !== editingId || $[45] !== editName || $[46] !== onLayerRename || $[47] !== commitEdit || $[48] !== handleKeyDown) {
    const t529 = useCallback;
    const t530 = () => {
      let t0;
      const t3 = editingId;
      t0 = t3;
      const t6 = editName;
      const t7 = t6.trim();
      t0 = t7;
      if (t0) {
        const t10 = onLayerRename;
        const t12 = editingId;
        const t14 = editName;
        const t15 = t14.trim();
        const t16 = t10(t12, t15);
      } else {
      }
      const t18 = setEditingId;
      const t19 = null;
      const t20 = t18(t19);
      const t21 = undefined;
      return t21;
    };
    const t531 = editingId;
    const t532 = editName;
    const t533 = onLayerRename;
    const t534 = [t531, t532, t533];
    const t535 = t529(t530, t534);
    commitEdit = t535;
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
    const t538 = useCallback;
    const t539 = (e) => {
      const t2 = e;
      const t3 = t2.key;
      const t4 = "Enter";
      const t5 = t3 === t4;
      if (t5) {
        const t7 = commitEdit;
        const t8 = t7();
      } else {
      }
      const t10 = e;
      const t11 = t10.key;
      const t12 = "Escape";
      const t13 = t11 === t12;
      if (t13) {
        const t15 = setEditingId;
        const t16 = null;
        const t17 = t15(t16);
      } else {
      }
      const t18 = undefined;
      return t18;
    };
    const t540 = commitEdit;
    const t541 = [t540];
    const t542 = t538(t539, t541);
    handleKeyDown = t542;
    $[49] = useCallback;
    $[50] = commitEdit;
    $[51] = handleKeyDown;
    $[52] = handleDragStart;
  } else {
  }
  const t545 = useCallback;
  const t546 = (index) => {
    const t2 = setDragIndex;
    const t4 = index;
    const t5 = t2(t4);
    const t6 = undefined;
    return t6;
  };
  const t547 = [];
  const t548 = t545(t546, t547);
  handleDragStart = t548;
  let handleDragOver;
  if ($[53] !== handleDragOver) {
    $[53] = handleDragOver;
  } else {
  }
  let handleDragEnd;
  if ($[54] !== useCallback || $[55] !== dragIndex || $[56] !== onLayerReorder || $[57] !== handleDragOver || $[58] !== handleDragEnd) {
    const t551 = useCallback;
    const t552 = (e, index) => {
      const t3 = e;
      const t4 = t3.preventDefault();
      let t5;
      const t8 = dragIndex;
      const t9 = null;
      const t10 = t8 !== t9;
      t5 = t10;
      const t13 = dragIndex;
      const t15 = index;
      const t16 = t13 !== t15;
      t5 = t16;
      if (t5) {
        const t19 = onLayerReorder;
        const t21 = dragIndex;
        const t23 = index;
        const t24 = t19(t21, t23);
        const t26 = setDragIndex;
        const t28 = index;
        const t29 = t26(t28);
      } else {
      }
      const t30 = undefined;
      return t30;
    };
    const t553 = dragIndex;
    const t554 = onLayerReorder;
    const t555 = [t553, t554];
    const t556 = t551(t552, t555);
    handleDragOver = t556;
    $[54] = useCallback;
    $[55] = dragIndex;
    $[56] = onLayerReorder;
    $[57] = handleDragOver;
    $[58] = handleDragEnd;
  } else {
  }
  const t559 = useCallback;
  const t560 = () => {
    const t1 = setDragIndex;
    const t2 = null;
    const t3 = t1(t2);
    const t4 = undefined;
    return t4;
  };
  const t561 = [];
  const t562 = t559(t560, t561);
  handleDragEnd = t562;
  let t564;
  if ($[59] !== collapsed) {
    t564 = collapsed;
    $[60] = t564;
    $[59] = collapsed;
  } else {
    t564 = $[60];
  }
  if (t564) {
    const t1037 = "div";
    const t1038 = "w-10 bg-white border-l flex flex-col items-center py-2";
    const t1039 = "button";
    const t1040 = () => {
      const t1 = setCollapsed;
      const t2 = false;
      const t3 = t1(t2);
      return t3;
    };
    const t1041 = "text-gray-500 hover:text-gray-700";
    const t1042 = "\n          ◀\n        ";
    const t1043 = _jsx(t1039, { onClick: t1040, className: t1041, children: t1042 });
    const t1044 = _jsx(t1037, { className: t1038, children: t1043 });
    return t1044;
  } else {
  }
  const t565 = "div";
  const t566 = "w-64 bg-white border-l flex flex-col h-full";
  const t567 = "div";
  const t568 = "flex items-center justify-between px-3 py-2 border-b";
  const t569 = "div";
  const t570 = "flex gap-1";
  const t571 = "layers";
  const t572 = "properties";
  const t573 = "history";
  const t574 = [t571, t572, t573];
  const t575 = (tab) => {
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
    let t10;
    if (t9) {
      const t12 = "bg-blue-100 text-blue-700";
      t10 = t12;
    } else {
      const t14 = "text-gray-500 hover:bg-gray-100";
      t10 = t14;
    }
    const t16 = `px-2 py-1 text-xs rounded ${t10}`;
    const t18 = tab;
    const t19 = 0;
    const t20 = t18.charAt(t19);
    const t21 = t20.toUpperCase();
    const t23 = tab;
    const t24 = 1;
    const t25 = t23.slice(t24);
    const t26 = t21 + t25;
    const t27 = _jsx(t1, { key: t3, onClick: t4, className: t16, children: t26 });
    return t27;
  };
  const t576 = t574.map(t575);
  const t577 = _jsx(t569, { className: t570, children: t576 });
  const t578 = "button";
  const t579 = () => {
    const t1 = setCollapsed;
    const t2 = true;
    const t3 = t1(t2);
    return t3;
  };
  const t580 = "text-gray-400 hover:text-gray-600";
  const t581 = "\n          ▶\n        ";
  const t582 = _jsx(t578, { onClick: t579, className: t580, children: t581 });
  const t583 = _jsxs(t567, { className: t568, children: [t577, t582] });
  let t233;
  if ($[61] !== searchQuery || $[62] !== filteredLayers || $[63] !== stats || $[64] !== stats || $[65] !== layers || $[66] !== onLayerAdd || $[67] !== activeLayer || $[68] !== t284 || $[69] !== layers || $[70] !== activeLayer || $[71] !== t308 || $[72] !== activeLayer || $[73] !== activeLayer || $[74] !== t233 || $[75] !== activeTab) {
    const t585 = activeTab;
    const t586 = "layers";
    const t587 = t585 === t586;
    t233 = t587;
    $[61] = searchQuery;
    $[62] = filteredLayers;
    $[63] = stats;
    $[64] = stats;
    $[65] = layers;
    $[66] = onLayerAdd;
    $[67] = activeLayer;
    $[68] = t284;
    $[69] = layers;
    $[70] = activeLayer;
    $[71] = t308;
    $[72] = activeLayer;
    $[73] = activeLayer;
    $[74] = t233;
    $[75] = activeTab;
  } else {
  }
  const t589 = "div";
  const t590 = "px-3 py-2 border-b";
  const t591 = "input";
  const t592 = searchQuery;
  const t593 = (e) => {
    const t2 = setSearchQuery;
    const t4 = e;
    const t5 = t4.target;
    const t6 = t5.value;
    const t7 = t2(t6);
    return t7;
  };
  const t594 = "Search layers...";
  const t595 = "w-full text-xs border rounded px-2 py-1";
  const t596 = _jsx(t591, { value: t592, onChange: t593, placeholder: t594, className: t595 });
  const t597 = _jsx(t589, { className: t590, children: t596 });
  const t598 = "div";
  const t599 = "flex-1 overflow-y-auto";
  const t600 = filteredLayers;
  const t601 = (layer, index) => {
    const t2 = "div";
    const t4 = layer;
    const t5 = t4.id;
    const t6 = true;
    const t7 = () => {
      const t1 = handleDragStart;
      const t3 = index;
      const t4 = t1(t3);
      return t4;
    };
    const t8 = (e) => {
      const t2 = handleDragOver;
      const t4 = e;
      const t6 = index;
      const t7 = t2(t4, t6);
      return t7;
    };
    const t10 = handleDragEnd;
    const t11 = () => {
      const t1 = onLayerSelect;
      const t3 = layer;
      const t4 = t3.id;
      const t5 = t1(t4);
      return t5;
    };
    const t13 = layer;
    const t14 = t13.id;
    const t16 = activeLayerId;
    const t17 = t14 === t16;
    let t18;
    if (t17) {
      const t20 = "bg-blue-50";
      t18 = t20;
    } else {
      const t22 = "hover:bg-gray-50";
      t18 = t22;
    }
    const t25 = dragIndex;
    const t27 = index;
    const t28 = t25 === t27;
    let t29;
    if (t28) {
      const t31 = "opacity-50";
      t29 = t31;
    } else {
      const t33 = "";
      t29 = t33;
    }
    const t35 = `flex items-center px-3 py-2 text-sm cursor-pointer border-b ${t18} ${t29}`;
    const t36 = "button";
    const t37 = (e) => {
      const t2 = e;
      const t3 = t2.stopPropagation();
      const t5 = onLayerToggleVisible;
      const t7 = layer;
      const t8 = t7.id;
      const t9 = t5(t8);
      const t10 = undefined;
      return t10;
    };
    const t38 = "mr-2 text-xs";
    const t40 = layer;
    const t41 = t40.visible;
    let t42;
    if (t41) {
      const t44 = "👁";
      t42 = t44;
    } else {
      const t46 = "○";
      t42 = t46;
    }
    const t48 = _jsx(t36, { onClick: t37, className: t38, children: t42 });
    const t50 = editingId;
    const t52 = layer;
    const t53 = t52.id;
    const t54 = t50 === t53;
    let t55;
    if (t54) {
      const t57 = "input";
      const t59 = editInputRef;
      const t61 = editName;
      const t62 = (e) => {
        const t2 = setEditName;
        const t4 = e;
        const t5 = t4.target;
        const t6 = t5.value;
        const t7 = t2(t6);
        return t7;
      };
      const t64 = commitEdit;
      const t66 = handleKeyDown;
      const t67 = "flex-1 text-xs border rounded px-1";
      const t68 = _jsx(t57, { ref: t59, value: t61, onChange: t62, onBlur: t64, onKeyDown: t66, className: t67 });
      t55 = t68;
    } else {
      const t70 = "span";
      const t71 = () => {
        const t1 = startEditing;
        const t3 = layer;
        const t4 = t1(t3);
        return t4;
      };
      const t73 = layer;
      const t74 = t73.visible;
      const t75 = !t74;
      let t76;
      if (t75) {
        const t78 = "text-gray-400";
        t76 = t78;
      } else {
        const t80 = "";
        t76 = t80;
      }
      const t82 = `flex-1 truncate ${t76}`;
      const t84 = layer;
      const t85 = t84.name;
      const t86 = _jsx(t70, { onDoubleClick: t71, className: t82, children: t85 });
      t55 = t86;
    }
    const t88 = "span";
    const t89 = "text-xs text-gray-400 ml-1";
    const t91 = layer;
    const t92 = t91.elements;
    const t93 = _jsx(t88, { className: t89, children: t92 });
    const t94 = "button";
    const t95 = (e) => {
      const t2 = e;
      const t3 = t2.stopPropagation();
      const t5 = onLayerToggleLock;
      const t7 = layer;
      const t8 = t7.id;
      const t9 = t5(t8);
      const t10 = undefined;
      return t10;
    };
    const t96 = "ml-1 text-xs";
    const t98 = layer;
    const t99 = t98.locked;
    let t100;
    if (t99) {
      const t102 = "🔒";
      t100 = t102;
    } else {
      const t104 = "🔓";
      t100 = t104;
    }
    const t106 = _jsx(t94, { onClick: t95, className: t96, children: t100 });
    const t107 = _jsxs(t2, { key: t5, draggable: t6, onDragStart: t7, onDragOver: t8, onDragEnd: t10, onClick: t11, className: t35, children: [t48, t55, t93, t106] });
    return t107;
  };
  const t602 = t600.map(t601);
  const t603 = _jsx(t598, { className: t599, children: t602 });
  const t604 = "div";
  const t605 = "px-3 py-2 border-t";
  const t606 = "div";
  const t607 = "flex justify-between items-center";
  const t608 = "div";
  const t609 = "text-xs text-gray-500";
  const t610 = stats;
  const t611 = t610.totalElements;
  const t612 = " elements · ";
  const t613 = stats;
  const t614 = t613.visibleLayers;
  const t615 = "/";
  const t616 = layers;
  const t617 = t616.length;
  const t618 = " visible\n              ";
  const t619 = _jsxs(t608, { className: t609, children: [t611, t612, t614, t615, t617, t618] });
  const t620 = "div";
  const t621 = "flex gap-1";
  const t622 = "button";
  const t623 = onLayerAdd;
  const t624 = "text-xs px-2 py-1 bg-blue-500 text-white rounded";
  const t625 = "\n                  +\n                ";
  const t626 = _jsx(t622, { onClick: t623, className: t624, children: t625 });
  let t284;
  const t628 = activeLayer;
  t284 = t628;
  const t630 = "button";
  const t631 = () => {
    const t1 = onLayerDuplicate;
    const t3 = activeLayerId;
    const t4 = t1(t3);
    return t4;
  };
  const t632 = "text-xs px-2 py-1 border rounded";
  const t633 = "\n                      ⧉\n                    ";
  const t634 = _jsx(t630, { onClick: t631, className: t632, children: t633 });
  const t635 = "button";
  const t636 = () => {
    const t1 = onLayerDelete;
    const t3 = activeLayerId;
    const t4 = t1(t3);
    return t4;
  };
  const t637 = "text-xs px-2 py-1 border rounded text-red-500";
  const t638 = layers;
  const t639 = t638.length;
  const t640 = 1;
  const t641 = t639 <= t640;
  const t642 = "\n                      🗑\n                    ";
  const t643 = _jsx(t635, { onClick: t636, className: t637, disabled: t641, children: t642 });
  const t644 = _jsxs(_Fragment, { children: [t634, t643] });
  t284 = t644;
  const t662 = _jsxs(t620, { className: t621, children: [t626, t284] });
  const t663 = _jsxs(t606, { className: t607, children: [t619, t662] });
  let t308;
  const t665 = activeLayer;
  t308 = t665;
  const t667 = "div";
  const t668 = "mt-2";
  const t669 = "label";
  const t670 = "text-xs text-gray-500";
  const t671 = "Opacity";
  const t672 = _jsx(t669, { className: t670, children: t671 });
  const t673 = "input";
  const t674 = "range";
  const t675 = 0;
  const t676 = 100;
  const t677 = activeLayer;
  const t678 = t677.opacity;
  const t679 = 100;
  const t680 = t678 * t679;
  const t681 = (e) => {
    const t2 = onLayerOpacity;
    const t4 = activeLayerId;
    const t5 = parseInt;
    const t7 = e;
    const t8 = t7.target;
    const t9 = t8.value;
    const t10 = t5(t9);
    const t11 = 100;
    const t12 = t10 / t11;
    const t13 = t2(t4, t12);
    return t13;
  };
  const t682 = "w-full";
  const t683 = _jsx(t673, { type: t674, min: t675, max: t676, value: t680, onChange: t681, className: t682 });
  const t684 = "span";
  const t685 = "text-xs text-gray-400";
  const t686 = Math;
  const t687 = activeLayer;
  const t688 = t687.opacity;
  const t689 = 100;
  const t690 = t688 * t689;
  const t691 = t686.round(t690);
  const t692 = "%";
  const t693 = _jsxs(t684, { className: t685, children: [t691, t692] });
  const t694 = _jsxs(t667, { className: t668, children: [t672, t683, t693] });
  t308 = t694;
  const t725 = _jsxs(t604, { className: t605, children: [t663, t308] });
  const t726 = _jsxs(_Fragment, { children: [t597, t603, t725] });
  t233 = t726;
  let t347;
  if ($[76] !== activeTab || $[77] !== t347) {
    const t823 = activeTab;
    const t824 = "properties";
    const t825 = t823 === t824;
    t347 = t825;
    $[76] = activeTab;
    $[77] = t347;
  } else {
  }
  const t827 = "div";
  const t828 = "p-3 text-sm text-gray-500";
  if ($[78] !== activeLayer) {
    const t829 = activeLayer;
    $[78] = activeLayer;
  } else {
  }
  let t358;
  if (t829) {
    if ($[79] !== t385 || $[80] !== activeLayer || $[81] !== t399 || $[82] !== activeLayer || $[83] !== t358 || $[84] !== activeLayer || $[85] !== activeLayer || $[86] !== activeLayer) {
      const t831 = "div";
      const t832 = "space-y-2";
      const t833 = "div";
      const t834 = "strong";
      const t835 = "Name:";
      const t836 = _jsx(t834, { children: t835 });
      const t837 = activeLayer;
      const t838 = t837.name;
      const t839 = _jsxs(t833, { children: [t836, t838] });
      const t840 = "div";
      const t841 = "strong";
      const t842 = "Elements:";
      const t843 = _jsx(t841, { children: t842 });
      const t844 = activeLayer;
      const t845 = t844.elements;
      const t846 = _jsxs(t840, { children: [t843, t845] });
      const t847 = "div";
      const t848 = "strong";
      const t849 = "Visible:";
      const t850 = _jsx(t848, { children: t849 });
      const t851 = activeLayer;
      const t852 = t851.visible;
      $[79] = t385;
      $[80] = activeLayer;
      $[81] = t399;
      $[82] = activeLayer;
      $[83] = t358;
      $[84] = activeLayer;
      $[85] = activeLayer;
      $[86] = activeLayer;
    } else {
    }
    let t385;
    if (t852) {
      const t889 = "Yes";
      t385 = t889;
    } else {
      const t891 = "No";
      t385 = t891;
    }
    const t858 = _jsxs(t847, { children: [t850, t385] });
    const t859 = "div";
    const t860 = "strong";
    const t861 = "Locked:";
    const t862 = _jsx(t860, { children: t861 });
    const t863 = activeLayer;
    const t864 = t863.locked;
    let t399;
    if (t864) {
      const t885 = "Yes";
      t399 = t885;
    } else {
      const t887 = "No";
      t399 = t887;
    }
    const t870 = _jsxs(t859, { children: [t862, t399] });
    const t871 = "div";
    const t872 = "strong";
    const t873 = "Opacity:";
    const t874 = _jsx(t872, { children: t873 });
    const t875 = Math;
    const t876 = activeLayer;
    const t877 = t876.opacity;
    const t878 = 100;
    const t879 = t877 * t878;
    const t880 = t875.round(t879);
    const t881 = "%";
    const t882 = _jsxs(t871, { children: [t874, t880, t881] });
    const t883 = _jsxs(t831, { className: t832, children: [t839, t846, t858, t870, t882] });
    t358 = t883;
  } else {
    const t893 = "p";
    const t894 = "Select a layer to view properties";
    const t895 = _jsx(t893, { children: t894 });
    t358 = t895;
  }
  const t955 = _jsx(t827, { className: t828, children: t358 });
  t347 = t955;
  let t427;
  if ($[87] !== activeTab || $[88] !== t427) {
    const t1022 = activeTab;
    const t1023 = "history";
    const t1024 = t1022 === t1023;
    t427 = t1024;
    $[87] = activeTab;
    $[88] = t427;
  } else {
  }
  const t1032 = "div";
  const t1033 = "p-3 text-sm text-gray-500 text-center";
  const t1034 = "\n          History panel (not implemented)\n        ";
  const t1035 = _jsx(t1032, { className: t1033, children: t1034 });
  t427 = t1035;
  const t1031 = _jsxs(t565, { className: t566, children: [t583, t233, t347, t427] });
  return t1031;
}

