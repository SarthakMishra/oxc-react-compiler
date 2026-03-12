import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
import { useState, useCallback } from 'react';

interface Todo {
  id: number;
  text: string;
  done: boolean;
}

export function TodoList() {
  const $ = _c(17);
  const t83 = useState;
  const t84 = [];
  const t85 = t83(t84);
  let todos;
  let setTodos;
  if ($[0] !== todos || $[1] !== setTodos) {
    $[0] = todos;
    $[1] = setTodos;
  } else {
  }
  ([todos, setTodos] = t85);
  const t89 = useState;
  const t90 = "";
  const t91 = t89(t90);
  let input;
  let setInput;
  if ($[2] !== input || $[3] !== setInput) {
    $[2] = input;
    $[3] = setInput;
  } else {
  }
  ([input, setInput] = t91);
  let addTodo;
  if ($[4] !== addTodo) {
    $[4] = addTodo;
  } else {
  }
  let toggleTodo;
  if ($[5] !== useCallback || $[6] !== input || $[7] !== addTodo || $[8] !== toggleTodo) {
    const t96 = useCallback;
    const t97 = () => {
      const t1 = input;
      const t2 = t1.trim();
      const t4 = setTodos;
      const t5 = (prev) => {
        const t2 = prev;
        const t3 = Date;
        const t4 = t3.now();
        const t6 = input;
        const t7 = false;
        const t8 = { id: t4, text: t6, done: t7 };
        const t9 = [...t2, t8];
        return t9;
      };
      const t6 = t4(t5);
      const t8 = setInput;
      const t9 = "";
      const t10 = t8(t9);
      const t11 = undefined;
      return t11;
    };
    const t98 = input;
    const t99 = [t98];
    const t100 = t96(t97, t99);
    addTodo = t100;
    $[5] = useCallback;
    $[6] = input;
    $[7] = addTodo;
    $[8] = toggleTodo;
  } else {
  }
  const t103 = useCallback;
  const t104 = (id) => {
    const t2 = setTodos;
    const t3 = (prev) => {
      const t2 = prev;
      const t3 = (t) => {
        const t2 = t;
        const t3 = t2.id;
        const t5 = id;
        const t6 = t3 === t5;
        const t8 = t;
        const t10 = t;
        const t11 = t10.done;
        const t12 = !t11;
        const t13 = { ...t8, done: t12 };
        const t15 = t;
        return t16;
      };
      const t4 = t2.map(t3);
      return t4;
    };
    const t4 = t2(t3);
    const t5 = undefined;
    return t5;
  };
  const t105 = [];
  const t106 = t103(t104, t105);
  toggleTodo = t106;
  let removeTodo;
  if ($[9] !== removeTodo) {
    $[9] = removeTodo;
  } else {
  }
  const t109 = useCallback;
  const t110 = (id) => {
    const t2 = setTodos;
    const t3 = (prev) => {
      const t2 = prev;
      const t3 = (t) => {
        const t2 = t;
        const t3 = t2.id;
        const t5 = id;
        const t6 = t3 !== t5;
        return t6;
      };
      const t4 = t2.filter(t3);
      return t4;
    };
    const t4 = t2(t3);
    const t5 = undefined;
    return t5;
  };
  const t111 = [];
  const t112 = t109(t110, t111);
  removeTodo = t112;
  let remaining;
  let t139;
  if ($[10] !== remaining || $[11] !== todos || $[12] !== remaining || $[13] !== input || $[14] !== addTodo || $[15] !== todos) {
    const t115 = todos;
    const t116 = (t) => {
      const t2 = t;
      const t3 = t2.done;
      const t4 = !t3;
      return t4;
    };
    const t117 = t115.filter(t116);
    const t118 = t117.length;
    remaining = t118;
    const t120 = "div";
    const t121 = "h2";
    const t122 = "Todos (";
    const t123 = remaining;
    const t124 = " remaining)";
    const t125 = _jsxs(t121, { children: [t122, t123, t124] });
    const t126 = "input";
    const t127 = input;
    const t128 = (e) => {
      const t2 = setInput;
      const t4 = e;
      const t5 = t4.target;
      const t6 = t5.value;
      const t7 = t2(t6);
      return t7;
    };
    const t129 = _jsx(t126, { value: t127, onChange: t128 });
    const t130 = "button";
    const t131 = addTodo;
    const t132 = "Add";
    const t133 = _jsx(t130, { onClick: t131, children: t132 });
    const t134 = "ul";
    const t135 = todos;
    const t136 = (todo) => {
      const t1 = "li";
      const t3 = todo;
      const t4 = t3.id;
      const t6 = todo;
      const t7 = t6.done;
      const t8 = "line-through";
      const t9 = "none";
      const t11 = { textDecoration: t10 };
      const t12 = "span";
      const t13 = () => {
        const t1 = toggleTodo;
        const t3 = todo;
        const t4 = t3.id;
        const t5 = t1(t4);
        return t5;
      };
      const t15 = todo;
      const t16 = t15.text;
      const t17 = _jsx(t12, { onClick: t13, children: t16 });
      const t18 = "button";
      const t19 = () => {
        const t1 = removeTodo;
        const t3 = todo;
        const t4 = t3.id;
        const t5 = t1(t4);
        return t5;
      };
      const t20 = "x";
      const t21 = _jsx(t18, { onClick: t19, children: t20 });
      const t22 = _jsxs(t1, { key: t4, style: t11, children: [t17, t21] });
      return t22;
    };
    const t137 = t135.map(t136);
    const t138 = _jsx(t134, { children: t137 });
    t139 = _jsxs(t120, { children: [t125, t129, t133, t138] });
    $[16] = t139;
    $[10] = remaining;
    $[11] = todos;
    $[12] = remaining;
    $[13] = input;
    $[14] = addTodo;
    $[15] = todos;
  } else {
    t139 = $[16];
  }
  return t139;
}

