export function calendarStatusLabel(todo) {
  if (todo.calendarSyncEnabled && todo.calendarSyncStatus === 'synced') return 'Calendar synced';
  if (todo.calendarSyncEnabled && todo.calendarSyncStatus === 'error') return 'Calendar error';
  if (todo.calendarSyncStatus === 'disabled') return 'Calendar off';
  return 'Calendar off';
}

export function calendarStatusClass(todo) {
  if (todo.calendarSyncEnabled && todo.calendarSyncStatus === 'synced') return 'calendar-synced';
  if (todo.calendarSyncEnabled && todo.calendarSyncStatus === 'error') return 'calendar-error';
  return 'calendar-off';
}

export function calendarActionLabel(todo) {
  if (todo.calendarSyncEnabled && todo.calendarSyncStatus === 'error') return 'Retry sync';
  return todo.calendarSyncEnabled ? 'Unsync calendar' : 'Sync calendar';
}

export function nextCalendarSyncState(todo) {
  if (todo.calendarSyncEnabled && todo.calendarSyncStatus === 'error') return true;
  return !todo.calendarSyncEnabled;
}
