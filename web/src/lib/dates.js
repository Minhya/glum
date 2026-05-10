const dateTimeFormatter = new Intl.DateTimeFormat(undefined, {
  month: 'short',
  day: 'numeric',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
});

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  month: 'short',
  day: 'numeric',
  year: 'numeric',
});

export function timestampToDate(timestamp) {
  if (!timestamp) return null;

  const seconds = Number(timestamp.seconds || 0);
  const nanos = Number(timestamp.nanos || 0);
  const date = new Date((seconds * 1000) + Math.floor(nanos / 1_000_000));

  return Number.isNaN(date.getTime()) ? null : date;
}

export function formatTimestamp(timestamp) {
  const date = timestampToDate(timestamp);
  return date ? dateTimeFormatter.format(date) : '';
}

export function formatDate(timestamp) {
  const date = timestampToDate(timestamp);
  return date ? dateFormatter.format(date) : '';
}

export function formatDue(todo) {
  const date = timestampToDate(todo.dueDate);
  if (!date) return '';

  return todo.dueHasTime ? dateTimeFormatter.format(date) : dateFormatter.format(date);
}

export function dateInputValue(timestamp) {
  const date = timestampToDate(timestamp);
  if (!date) return '';

  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

export function timeInputValue(timestamp) {
  const date = timestampToDate(timestamp);
  if (!date) return '';

  const hours = String(date.getHours()).padStart(2, '0');
  const minutes = String(date.getMinutes()).padStart(2, '0');
  return `${hours}:${minutes}`;
}

export function todoDueValue(date, time) {
  if (!date) return '';
  return time ? `${date}T${time}` : date;
}
