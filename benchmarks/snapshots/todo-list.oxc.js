import { c as _c } from "react/compiler-runtime";
import { useState, useCallback } from 'react';

interface Todo {
  id: number;
  text: string;
  done: boolean;
}

export function TodoList() {
  const $ = _c(17);
  let t2;
  let t8;
  let t62;
  let t63;
  let t25;
  let t26;
  let t64;
  let t65;
  let t32;
  let t33;
  let t66;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t2 = [];
    $[0] = t2;
  } else {
    t2 = $[0];
  }
  let t3 = useState(t2);
  let todos;
  let setTodos;
  ([todos, setTodos] = t3);
  if ($[1] !== t3) {
    $[1] = t3;
    $[2] = todos;
    $[3] = setTodos;
  } else {
    todos = $[2];
    setTodos = $[3];
  }
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    t8 = "";
    $[4] = t8;
  } else {
    t8 = $[4];
  }
  let input;
  let setInput;
  ([input, setInput] = useState(t8));
  let addTodo;
  let t17 = () => {
    if (input.trim()) {
      let t5 = (prev) => {
        return [...prev, { id: Date.now(), text: input, done: false }];
      };
      let t6 = setTodos(t5);
      let t10 = setInput("");
    }
    return undefined;
  };
  let t20 = useCallback(t17, [input]);
  if ($[5] !== t20) {
    t62 = t20;
    $[5] = t20;
    $[6] = t62;
  } else {
    t62 = $[6];
  }
  addTodo = t62;
  if ($[7] === Symbol.for("react.memo_cache_sentinel")) {
    t25 = (id) => {
      let t3 = (prev) => {
        let t2 = (t) => {
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
      let t4 = setTodos(t3);
      return undefined;
    };
    t26 = [];
    $[7] = t63;
    $[8] = t25;
    $[9] = t26;
  } else {
    t63 = $[7];
    t25 = $[8];
    t26 = $[9];
  }
  let toggleTodo = t63;
  let t27 = useCallback(t25, t26);
  if ($[10] !== t27) {
    t64 = t27;
    $[10] = t27;
    $[11] = t64;
  } else {
    t64 = $[11];
  }
  toggleTodo = t64;
  if ($[12] === Symbol.for("react.memo_cache_sentinel")) {
    t32 = (id) => {
      let t3 = (prev) => {
        let t2 = (t) => {
          return t.id !== id;
        };
        return prev.filter(t2);
      };
      let t4 = setTodos(t3);
      return undefined;
    };
    t33 = [];
    $[12] = t65;
    $[13] = t32;
    $[14] = t33;
  } else {
    t65 = $[12];
    t32 = $[13];
    t33 = $[14];
  }
  let removeTodo = t65;
  let t34 = useCallback(t32, t33);
  if ($[15] !== t34) {
    t66 = t34;
    $[15] = t34;
    $[16] = t66;
  } else {
    t66 = $[16];
  }
  removeTodo = t66;
  let remaining;
  let t38 = (t) => {
    return !t.done;
  };
  remaining = todos.filter(t38).length;
  let t49 = (e) => {
    return setInput(e.target.value);
  };
  let t57 = (todo) => {
    let t6;
    if (todo.done) {
      t6 = "line-through";
    } else {
      t6 = "none";
    }
    let t11 = () => {
      return toggleTodo(todo.id);
    };
    let t16 = () => {
      return removeTodo(todo.id);
    };
    return <li key={todo.id} style={{ textDecoration: t6 }}><span onClick={t11}>{todo.text}</span><button onClick={t16}>x</button></li>;
  };
  return <div><h2>Todos ({remaining} remaining)</h2><input value={input} onChange={t49} /><button onClick={addTodo}>Add</button><ul>{todos.map(t57)}</ul></div>;
}

