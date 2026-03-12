import { c as _c } from "react/compiler-runtime";
import { useState, useCallback } from 'react';

interface Todo {
  id: number;
  text: string;
  done: boolean;
}

export function TodoList() {
  const $ = _c(7);
  const t83 = useState;
  const t84 = [];
  const t85 = t83(t84);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t86 = Discriminant(4) */
    /* t87 = Discriminant(4) */
  } else {
  }
  /* t88 = Discriminant(6) */
  const t89 = useState;
  const t90 = "";
  const t91 = t89(t90);
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    /* t92 = Discriminant(4) */
    /* t93 = Discriminant(4) */
  } else {
  }
  /* t94 = Discriminant(6) */
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    /* t95 = Discriminant(4) */
  } else {
  }
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    const t96 = useCallback;
    /* t97 = Discriminant(28) */
    const t98 = input;
    const t99 = [t98];
    const t100 = t96(t97, t99);
    const addTodo = t100;
    /* t102 = Discriminant(4) */
  } else {
  }
  const t103 = useCallback;
  /* t104 = Discriminant(28) */
  const t105 = [];
  const t106 = t103(t104, t105);
  const toggleTodo = t106;
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    /* t108 = Discriminant(4) */
  } else {
  }
  const t109 = useCallback;
  /* t110 = Discriminant(28) */
  const t111 = [];
  const t112 = t109(t110, t111);
  const removeTodo = t112;
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    /* t114 = Discriminant(4) */
    const t115 = todos;
    /* t116 = Discriminant(28) */
    const t117 = t115.filter(t116);
    const t118 = t117.length;
    const remaining = t118;
    const t120 = "div";
    const t121 = "h2";
    /* t122 = Discriminant(8) */
    const t123 = remaining;
    /* t124 = Discriminant(8) */
    const t125 = <t121>{t122}{t123}{t124}</t121>;
    const t126 = "input";
    const t127 = input;
    /* t128 = Discriminant(28) */
    const t129 = <t126 value={t127} onChange={t128} />;
    const t130 = "button";
    const t131 = addTodo;
    /* t132 = Discriminant(8) */
    const t133 = <t130 onClick={t131}>{t132}</t130>;
    const t134 = "ul";
    const t135 = todos;
    /* t136 = Discriminant(28) */
    const t137 = t135.map(t136);
    const t138 = <t134>{t137}</t134>;
    const t139 = <t120>{t125}{t129}{t133}{t138}</t120>;
    $[6] = t139;
  } else {
    t139 = $[6];
  }
  return t139;
}

