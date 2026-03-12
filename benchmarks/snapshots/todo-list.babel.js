import { c as _c } from "react/compiler-runtime";
import { useState, useCallback } from 'react';
import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
export function TodoList() {
  const $ = _c(24);
  let t0;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t0 = [];
    $[0] = t0;
  } else {
    t0 = $[0];
  }
  const [todos, setTodos] = useState(t0);
  const [input, setInput] = useState("");
  let t1;
  if ($[1] !== input) {
    t1 = () => {
      if (input.trim()) {
        setTodos(prev => [...prev, {
          id: Date.now(),
          text: input,
          done: false
        }]);
        setInput("");
      }
    };
    $[1] = input;
    $[2] = t1;
  } else {
    t1 = $[2];
  }
  const addTodo = t1;
  let t2;
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    t2 = id => {
      setTodos(prev_0 => prev_0.map(t => t.id === id ? {
        ...t,
        done: !t.done
      } : t));
    };
    $[3] = t2;
  } else {
    t2 = $[3];
  }
  const toggleTodo = t2;
  let t3;
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    t3 = id_0 => {
      setTodos(prev_1 => prev_1.filter(t_0 => t_0.id !== id_0));
    };
    $[4] = t3;
  } else {
    t3 = $[4];
  }
  const removeTodo = t3;
  let t4;
  if ($[5] !== todos) {
    t4 = todos.filter(_temp);
    $[5] = todos;
    $[6] = t4;
  } else {
    t4 = $[6];
  }
  const remaining = t4.length;
  let t5;
  if ($[7] !== remaining) {
    t5 = /*#__PURE__*/_jsxs("h2", {
      children: ["Todos (", remaining, " remaining)"]
    });
    $[7] = remaining;
    $[8] = t5;
  } else {
    t5 = $[8];
  }
  let t6;
  if ($[9] === Symbol.for("react.memo_cache_sentinel")) {
    t6 = e => setInput(e.target.value);
    $[9] = t6;
  } else {
    t6 = $[9];
  }
  let t7;
  if ($[10] !== input) {
    t7 = /*#__PURE__*/_jsx("input", {
      value: input,
      onChange: t6
    });
    $[10] = input;
    $[11] = t7;
  } else {
    t7 = $[11];
  }
  let t8;
  if ($[12] !== addTodo) {
    t8 = /*#__PURE__*/_jsx("button", {
      onClick: addTodo,
      children: "Add"
    });
    $[12] = addTodo;
    $[13] = t8;
  } else {
    t8 = $[13];
  }
  let t9;
  if ($[14] !== todos) {
    let t10;
    if ($[16] === Symbol.for("react.memo_cache_sentinel")) {
      t10 = todo => /*#__PURE__*/_jsxs("li", {
        style: {
          textDecoration: todo.done ? "line-through" : "none"
        },
        children: [/*#__PURE__*/_jsx("span", {
          onClick: () => toggleTodo(todo.id),
          children: todo.text
        }), /*#__PURE__*/_jsx("button", {
          onClick: () => removeTodo(todo.id),
          children: "x"
        })]
      }, todo.id);
      $[16] = t10;
    } else {
      t10 = $[16];
    }
    t9 = todos.map(t10);
    $[14] = todos;
    $[15] = t9;
  } else {
    t9 = $[15];
  }
  let t10;
  if ($[17] !== t9) {
    t10 = /*#__PURE__*/_jsx("ul", {
      children: t9
    });
    $[17] = t9;
    $[18] = t10;
  } else {
    t10 = $[18];
  }
  let t11;
  if ($[19] !== t10 || $[20] !== t5 || $[21] !== t7 || $[22] !== t8) {
    t11 = /*#__PURE__*/_jsxs("div", {
      children: [t5, t7, t8, t10]
    });
    $[19] = t10;
    $[20] = t5;
    $[21] = t7;
    $[22] = t8;
    $[23] = t11;
  } else {
    t11 = $[23];
  }
  return t11;
}
function _temp(t_1) {
  return !t_1.done;
}