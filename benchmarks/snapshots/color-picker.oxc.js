import { c as _c } from "react/compiler-runtime";
// M tier - Inspired by excalidraw ColorPicker component
import { useState, useCallback, useMemo, useRef, useEffect } from 'react';

interface ColorPickerProps {
  color: string;
  onChange: (color: string) => void;
  presets?: string[];
  showCustom?: boolean;
}

const DEFAULT_PRESETS = [
  '#000000', '#545454', '#a0a0a0', '#ffffff',
  '#e03131', '#e8590c', '#fcc419', '#40c057',
  '#228be6', '#7048e8', '#be4bdb', '#f06595',
];

export function ColorPicker(t0) {
  const $ = _c(8);
  const t160 = useState;
  const t161 = false;
  const t162 = t160(t161);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t163 = Discriminant(4) */
    /* t164 = Discriminant(4) */
  } else {
  }
  /* t165 = Discriminant(6) */
  const t166 = useState;
  const t167 = color;
  const t168 = t166(t167);
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    /* t169 = Discriminant(4) */
    /* t170 = Discriminant(4) */
  } else {
  }
  /* t171 = Discriminant(6) */
  const t172 = useState;
  const t173 = [];
  const t174 = t172(t173);
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    /* t175 = Discriminant(4) */
    /* t176 = Discriminant(4) */
  } else {
  }
  /* t177 = Discriminant(6) */
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    /* t178 = Discriminant(4) */
  } else {
  }
  const t179 = useRef;
  const t180 = null;
  const t181 = t179(t180);
  const popoverRef = t181;
  const t183 = useEffect;
  /* t184 = Discriminant(28) */
  const t185 = [];
  const t186 = t183(t184, t185);
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    /* t187 = Discriminant(4) */
  } else {
  }
  const t188 = useCallback;
  /* t189 = Discriminant(28) */
  const t190 = [];
  const t191 = t188(t189, t190);
  const addToRecent = t191;
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    /* t193 = Discriminant(4) */
  } else {
  }
  if ($[6] === Symbol.for("react.memo_cache_sentinel")) {
    const t194 = useCallback;
    /* t195 = Discriminant(28) */
    const t196 = onChange;
    const t197 = addToRecent;
    const t198 = [t196, t197];
    const t199 = t194(t195, t198);
    const handlePresetClick = t199;
    /* t201 = Discriminant(4) */
  } else {
  }
  if ($[7] === Symbol.for("react.memo_cache_sentinel")) {
    const t202 = useCallback;
    /* t203 = Discriminant(28) */
    const t204 = customColor;
    const t205 = onChange;
    const t206 = addToRecent;
    const t207 = [t204, t205, t206];
    const t208 = t202(t203, t207);
    const handleCustomSubmit = t208;
    /* t210 = Discriminant(4) */
  } else {
  }
  const t211 = useMemo;
  /* t212 = Discriminant(28) */
  const t213 = presets;
  const t214 = [t213];
  const t215 = t211(t212, t214);
  const groupedPresets = t215;
  const t217 = "div";
  const t218 = "relative inline-block";
  const t219 = popoverRef;
  const t220 = "button";
  /* t221 = Discriminant(28) */
  const t222 = "w-8 h-8 rounded border-2";
  const t223 = color;
  const t224 = { backgroundColor: t223 };
  const t225 = "Pick color";
  const t226 = <t220 onClick={t221} className={t222} style={t224} aria-label={t225} />;
}

