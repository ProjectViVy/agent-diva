import type { PlanRuntimeState, PlanRuntimeTodo } from '../../api/planning';

/** Terminal TODOs remain historical evidence, never active execution work. */
export function isTerminalTodoStatus(status: string): boolean {
  return ['completed', 'canceled', 'cancelled'].includes(status.toLowerCase());
}

export function activePlanTodos(todos: PlanRuntimeTodo[]): PlanRuntimeTodo[] {
  return todos.filter((todo) => !isTerminalTodoStatus(todo.status));
}

/**
 * Report-style plans are markdown documents and often have zero TODOs.
 * The chat TODO strip must not appear just because a plan is executing —
 * empty "无执行清单" chrome is noise and implies a false PLAN↔TODO coupling.
 */
export function shouldShowPlanTodoPanel(
  plan: Pick<PlanRuntimeState, 'todos'> | null | undefined,
): boolean {
  return (plan?.todos?.length ?? 0) > 0;
}
