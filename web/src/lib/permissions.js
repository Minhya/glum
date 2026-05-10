export function canEditItem(item) {
  return item.owner || item.canWrite !== false;
}

export function canManageItem(item) {
  return item.owner !== false;
}

export function accessLabel(item) {
  if (item.owner) return 'Owner';
  return canEditItem(item) ? 'Can edit' : 'View only';
}
