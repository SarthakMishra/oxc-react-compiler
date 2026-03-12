import { useState, useCallback } from 'react';

interface Todo {
  id: number;
  text: string;
  done: boolean;
}

export function TodoList() {
  const [todos, setTodos] = useState<Todo[]>([]);
  const [input, setInput] = useState('');

  const addTodo = useCallback(() => {
    if (input.trim()) {
      setTodos((prev) => [...prev, { id: Date.now(), text: input, done: false }]);
      setInput('');
    }
  }, [input]);

  const toggleTodo = useCallback((id: number) => {
    setTodos((prev) =>
      prev.map((t) => (t.id === id ? { ...t, done: !t.done } : t))
    );
  }, []);

  const removeTodo = useCallback((id: number) => {
    setTodos((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const remaining = todos.filter((t) => !t.done).length;

  return (
    <div>
      <h2>Todos ({remaining} remaining)</h2>
      <input value={input} onChange={(e) => setInput(e.target.value)} />
      <button onClick={addTodo}>Add</button>
      <ul>
        {todos.map((todo) => (
          <li key={todo.id} style={{ textDecoration: todo.done ? 'line-through' : 'none' }}>
            <span onClick={() => toggleTodo(todo.id)}>{todo.text}</span>
            <button onClick={() => removeTodo(todo.id)}>x</button>
          </li>
        ))}
      </ul>
    </div>
  );
}
