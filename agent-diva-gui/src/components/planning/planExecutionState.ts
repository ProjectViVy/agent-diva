import type { PlanRuntimeTodo } from '../../api/planning';

/** Terminal TODOs remain historical evidence, never active execution work. */
export function isTerminalTodoStatus(status: string): boolean {
  return ['completed', 'canceled', 'cancelled'].includes(status.toLowerCase());
}

export function activePlanTodos(todos: PlanRuntimeTodo[]): PlanRuntimeTodo[] {
  return todos.filter((todo) => !isTerminalTodoStatus(todo.status));
}
