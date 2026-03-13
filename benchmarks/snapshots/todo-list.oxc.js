import { c as _c } from "react/compiler-runtime";
import { useState, useCallback } from 'react';

interface Todo {
  id: number;
  text: string;
  done: boolean;
}

export function TodoList() {
  const $ = _c(10);
  let todos;
  let setTodos;
  if ($[0] !== useState) {
    $[0] = useState;
  }
  let toggleTodo;
  if ($[1] !== input || $[2] !== useCallback) {
    const t97 = () => {
      if (input.trim()) {
        const t5 = (prev) => {
          return [...prev, { id: Date.now(), text: input, done: false }];
        };
        const t6 = setTodos(t5);
        const t10 = setInput("");
      }
      return undefined;
    };
    const addTodo = useCallback(t97, [input]);
    $[1] = input;
    $[2] = useCallback;
  }
  let removeTodo;
  if ($[3] !== useCallback) {
    const t104 = (id) => {
      const t3 = (prev) => {
        const t3 = (t) => {
          if (t.id === id) {
            t7 = { ...t, done: !t.done };
          } else {
            t7 = t;
          }
          return t7;
        };
        return prev.map(t3);
      };
      const t4 = setTodos(t3);
      return undefined;
    };
    toggleTodo = useCallback(t104, []);
    $[3] = useCallback;
  }
  let remaining;
  let t139;
  if ($[4] !== addTodo || $[5] !== input || $[6] !== todos || $[7] !== todos || $[8] !== useCallback) {
    const t110 = (id) => {
      const t3 = (prev) => {
        const t3 = (t) => {
          return t.id !== id;
        };
        return prev.filter(t3);
      };
      const t4 = setTodos(t3);
      return undefined;
    };
    removeTodo = useCallback(t110, []);
    const t116 = (t) => {
      return !t.done;
    };
    remaining = todos.filter(t116).length;
    const t128 = (e) => {
      return setInput(e.target.value);
    };
    const t136 = (todo) => {
      if (todo.done) {
        t8 = "line-through";
      } else {
        t8 = "none";
      }
      const t16 = () => {
        return toggleTodo(todo.id);
      };
      const t22 = () => {
        return removeTodo(todo.id);
      };
      return <li key={todo.id} style={{ textDecoration: t8 }}><span onClick={t16}>{todo.text}</span><button onClick={t22}>x</button></li>;
    };
    t139 = (
      <div>
        <h2>Todos ({remaining} remaining)</h2>
        <input value={input} onChange={t128} />
        <button onClick={addTodo}>Add</button>
        <ul>{todos.map(t136)}</ul>
      </div>
    );
    $[4] = addTodo;
    $[5] = input;
    $[6] = todos;
    $[7] = todos;
    $[8] = useCallback;
    $[9] = t139;
  } else {
    t139 = $[9];
  }
  return t139;
}

