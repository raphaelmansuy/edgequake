/**
 * One way for any screen to ask "open the Create Workspace wizard".
 * The header owns the wizard; callers (for example the no-graph empty state) only ask.
 */
export const OPEN_CREATE_WORKSPACE_EVENT = 'eq:create-workspace';

export function requestCreateWorkspace(win: Pick<Window, 'dispatchEvent'> = window): void {
  win.dispatchEvent(new CustomEvent(OPEN_CREATE_WORKSPACE_EVENT));
}
