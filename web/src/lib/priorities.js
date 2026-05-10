export const priorities = [
  { value: 0, label: 'Low' },
  { value: 1, label: 'Medium' },
  { value: 2, label: 'High' },
];

export function priorityLabel(value) {
  return priorities.find((priority) => priority.value === Number(value))?.label || 'Low';
}

export function priorityClass(value) {
  return priorityLabel(value).toLowerCase();
}
