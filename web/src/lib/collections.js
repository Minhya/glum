export function ensureShareDefaults(item) {
  if (item.shareCanWrite === undefined) {
    item.shareCanWrite = true;
  }
}

export function replaceById(items, item) {
  return items.map((existing) => existing.id === item.id ? item : existing);
}
