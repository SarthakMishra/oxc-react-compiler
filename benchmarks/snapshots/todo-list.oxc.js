import { c as _c } from "react/compiler-runtime";
import { useState, useCallback } from 'react';

interface Todo {
  id: number;
  text: string;
  done: boolean;
}

export function TodoList() {
  const $ = _c(21);
  let t60;
  let t61;
  let t17;
  let t19;
  let t2;
  if ($[0] !== t3 || $[1] !== todos) {
    t2 = [];
    $[0] = t3;
    $[1] = todos;
    $[2] = t60;
    $[3] = t61;
    $[4] = t17;
    $[5] = t19;
    $[6] = t2;
  } else {
    t60 = $[2];
    t61 = $[3];
    t17 = $[4];
    t19 = $[5];
    t2 = $[6];
  }
  const addTodo = t61;
  let t62;
  if ($[7] === Symbol.for("react.memo_cache_sentinel")) {
    $[7] = t62;
  } else {
    t62 = $[7];
  }
  const todos = t62;
  let setTodos;
  let t8;
  if ($[8] === Symbol.for("react.memo_cache_sentinel")) {
    $[8] = t8;
  } else {
    t8 = $[8];
  }
  let addTodo;
  t17 = () => {
    if (input.trim()) {
      const t5 = (prev) => {
        return [...prev, { id: Date.now(), text: input, done: false }];
      };
      const t6 = setTodos(t5);
      const t10 = setInput("");
    }
    return undefined;
  };
  const t20 = useCallback(t17, [input]);
  let t63;
  if ($[9] !== t20) {
    t63 = t20;
    $[9] = t20;
    $[10] = t63;
  } else {
    t63 = $[10];
  }
  addTodo = t63;
  let t64;
  let t25;
  let t26;
  if ($[11] === Symbol.for("react.memo_cache_sentinel")) {
    t25 = (id) => {
      const t3 = (prev) => {
        const t2 = (t) => {
          let t6;
          if (t.id === id) {
            t6 = { ...t, done: !t.done };
          } else {
            t6 = t;
          }
          return t6;
        };
        return prev.map(t2);
      };
      const t4 = setTodos(t3);
      return undefined;
    };
    t26 = [];
    $[11] = t64;
    $[12] = t25;
    $[13] = t26;
  } else {
    t64 = $[11];
    t25 = $[12];
    t26 = $[13];
  }
  const toggleTodo = t64;
  const t27 = useCallback(t25, t26);
  let t65;
  if ($[14] !== t27) {
    t65 = t27;
    $[14] = t27;
    $[15] = t65;
  } else {
    t65 = $[15];
  }
  const toggleTodo = t65;
  let t66;
  let t32;
  let t33;
  if ($[16] === Symbol.for("react.memo_cache_sentinel")) {
    t32 = (id) => {
      const t3 = (prev) => {
        const t2 = (t) => {
          return t.id !== id;
        };
        return prev.filter(t2);
      };
      const t4 = setTodos(t3);
      return undefined;
    };
    t33 = [];
    $[16] = t66;
    $[17] = t32;
    $[18] = t33;
  } else {
    t66 = $[16];
    t32 = $[17];
    t33 = $[18];
  }
  const removeTodo = t66;
  const t34 = useCallback(t32, t33);
  let t67;
  if ($[19] !== t34) {
    t67 = t34;
    $[19] = t34;
    $[20] = t67;
  } else {
    t67 = $[20];
  }
  const removeTodo = t67;
  let remaining;
  const t38 = (t) => {
    return !t.done;
  };
  remaining = todos.filter(t38).length;
  const t49 = (e) => {
    return setInput(e.target.value);
  };
  const t57 = (todo) => {
    let t6;
    if (todo.done) {
      t6 = "line-through";
    } else {
      t6 = "none";
    }
    const t11 = () => {
      return toggleTodo(todo.id);
    };
    const t16 = () => {
      return removeTodo(todo.id);
    };
    return <li key={todo.id} style={{ textDecoration: t6 }}><span onClick={t11}>{todo.text}</span><button onClick={t16}>x</button></li>;
  };
  return <div><h2>Todos ({remaining} remaining)</h2><input value={input} onChange={t49} /><button onClick={addTodo}>Add</button><ul>{todos.map(t57)}</ul></div>;
}

