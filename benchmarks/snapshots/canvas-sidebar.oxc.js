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
  const $ = _c(20);
  const t394 = useState;
  const t395 = "layers";
  const t396 = t394(t395);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t397 = Discriminant(4) */
    /* t398 = Discriminant(4) */
  } else {
  }
  /* t399 = Discriminant(6) */
  const t400 = useState;
  const t401 = null;
  const t402 = t400(t401);
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    /* t403 = Discriminant(4) */
    /* t404 = Discriminant(4) */
  } else {
  }
  /* t405 = Discriminant(6) */
  const t406 = useState;
  const t407 = "";
  const t408 = t406(t407);
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    /* t409 = Discriminant(4) */
    /* t410 = Discriminant(4) */
  } else {
  }
  /* t411 = Discriminant(6) */
  const t412 = useState;
  const t413 = "";
  const t414 = t412(t413);
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    /* t415 = Discriminant(4) */
    /* t416 = Discriminant(4) */
  } else {
  }
  /* t417 = Discriminant(6) */
  const t418 = useState;
  const t419 = null;
  const t420 = t418(t419);
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    /* t421 = Discriminant(4) */
    /* t422 = Discriminant(4) */
  } else {
  }
  /* t423 = Discriminant(6) */
  const t424 = useState;
  const t425 = false;
  const t426 = t424(t425);
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    /* t427 = Discriminant(4) */
    /* t428 = Discriminant(4) */
  } else {
  }
  /* t429 = Discriminant(6) */
  if ($[6] === Symbol.for("react.memo_cache_sentinel")) {
    /* t430 = Discriminant(4) */
  } else {
  }
  const t431 = useRef;
  const t432 = null;
  const t433 = t431(t432);
  const editInputRef = t433;
  const t435 = useEffect;
  /* t436 = Discriminant(28) */
  if ($[7] === Symbol.for("react.memo_cache_sentinel")) {
    const t437 = editingId;
    const t438 = [t437];
    const t439 = t435(t436, t438);
    /* t440 = Discriminant(4) */
  } else {
  }
  if ($[8] === Symbol.for("react.memo_cache_sentinel")) {
    const t441 = useMemo;
    /* t442 = Discriminant(28) */
    const t443 = layers;
    const t444 = searchQuery;
    const t445 = [t443, t444];
    const t446 = t441(t442, t445);
    const filteredLayers = t446;
    /* t448 = Discriminant(4) */
  } else {
  }
  const t449 = useMemo;
  /* t450 = Discriminant(28) */
  const t451 = layers;
  const t452 = activeLayerId;
  const t453 = [t451, t452];
  const t454 = t449(t450, t453);
  const activeLayer = t454;
  if ($[9] === Symbol.for("react.memo_cache_sentinel")) {
    /* t456 = Discriminant(4) */
  } else {
  }
  const t457 = useMemo;
  /* t458 = Discriminant(28) */
  const t459 = layers;
  const t460 = [t459];
  const t461 = t457(t458, t460);
  const stats = t461;
  if ($[10] === Symbol.for("react.memo_cache_sentinel")) {
    /* t463 = Discriminant(4) */
  } else {
  }
  const t464 = useCallback;
  /* t465 = Discriminant(28) */
  const t466 = [];
  const t467 = t464(t465, t466);
  const startEditing = t467;
  if ($[11] === Symbol.for("react.memo_cache_sentinel")) {
    /* t469 = Discriminant(4) */
  } else {
  }
  if ($[12] === Symbol.for("react.memo_cache_sentinel")) {
    const t470 = useCallback;
    /* t471 = Discriminant(28) */
    const t472 = editingId;
    const t473 = editName;
    const t474 = onLayerRename;
    const t475 = [t472, t473, t474];
    const t476 = t470(t471, t475);
    const commitEdit = t476;
    /* t478 = Discriminant(4) */
  } else {
  }
  if ($[13] === Symbol.for("react.memo_cache_sentinel")) {
    const t479 = useCallback;
    /* t480 = Discriminant(28) */
    const t481 = commitEdit;
    const t482 = [t481];
    const t483 = t479(t480, t482);
    const handleKeyDown = t483;
    /* t485 = Discriminant(4) */
  } else {
  }
  const t486 = useCallback;
  /* t487 = Discriminant(28) */
  const t488 = [];
  const t489 = t486(t487, t488);
  const handleDragStart = t489;
  if ($[14] === Symbol.for("react.memo_cache_sentinel")) {
    /* t491 = Discriminant(4) */
  } else {
  }
  if ($[15] === Symbol.for("react.memo_cache_sentinel")) {
    const t492 = useCallback;
    /* t493 = Discriminant(28) */
    const t494 = dragIndex;
    const t495 = onLayerReorder;
    const t496 = [t494, t495];
    const t497 = t492(t493, t496);
    const handleDragOver = t497;
    /* t499 = Discriminant(4) */
  } else {
  }
  const t500 = useCallback;
  /* t501 = Discriminant(28) */
  const t502 = [];
  const t503 = t500(t501, t502);
  const handleDragEnd = t503;
  if ($[16] === Symbol.for("react.memo_cache_sentinel")) {
    const t505 = collapsed;
    $[17] = t505;
  } else {
    t505 = $[17];
  }
  if (t505) {
    const t922 = "div";
    const t923 = "w-10 bg-white border-l flex flex-col items-center py-2";
    const t924 = "button";
    /* t925 = Discriminant(28) */
    const t926 = "text-gray-500 hover:text-gray-700";
    /* t927 = Discriminant(8) */
    const t928 = <t924 onClick={t925} className={t926}>{t927}</t924>;
    const t929 = <t922 className={t923}>{t928}</t922>;
    return t929;
  } else {
    const t506 = "div";
    const t507 = "w-64 bg-white border-l flex flex-col h-full";
    const t508 = "div";
    const t509 = "flex items-center justify-between px-3 py-2 border-b";
    const t510 = "div";
    const t511 = "flex gap-1";
    const t512 = "layers";
    const t513 = "properties";
    const t514 = "history";
    const t515 = [t512, t513, t514];
    /* t516 = Discriminant(28) */
    const t517 = t515.map(t516);
    const t518 = <t510 className={t511}>{t517}</t510>;
    const t519 = "button";
    /* t520 = Discriminant(28) */
    const t521 = "text-gray-400 hover:text-gray-600";
    /* t522 = Discriminant(8) */
    const t523 = <t519 onClick={t520} className={t521}>{t522}</t519>;
    const t524 = <t508 className={t509}>{t518}{t523}</t508>;
    if ($[18] === Symbol.for("react.memo_cache_sentinel")) {
      const t525 = activeTab;
    } else {
    }
    const t526 = "layers";
  }
  const t506 = "div";
  const t507 = "w-64 bg-white border-l flex flex-col h-full";
  const t508 = "div";
  const t509 = "flex items-center justify-between px-3 py-2 border-b";
  const t510 = "div";
  const t511 = "flex gap-1";
  const t512 = "layers";
  const t513 = "properties";
  const t514 = "history";
  const t515 = [t512, t513, t514];
  /* t516 = Discriminant(28) */
  const t517 = t515.map(t516);
  const t518 = <t510 className={t511}>{t517}</t510>;
  const t519 = "button";
  /* t520 = Discriminant(28) */
  const t521 = "text-gray-400 hover:text-gray-600";
  /* t522 = Discriminant(8) */
  const t523 = <t519 onClick={t520} className={t521}>{t522}</t519>;
  const t524 = <t508 className={t509}>{t518}{t523}</t508>;
  if ($[19] === Symbol.for("react.memo_cache_sentinel")) {
    const t525 = activeTab;
  } else {
  }
  const t526 = "layers";
}

