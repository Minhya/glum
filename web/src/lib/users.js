export function userLabel(user) {
  const label = user.displayName || user.email || user.username || user.id;
  const detail = user.email && user.email !== label ? ` (${user.email})` : '';
  return `${label}${detail}`;
}

export function userName(user) {
  return user.displayName || user.username || user.email || user.id;
}

export function userDetail(user) {
  return user.email && user.email !== userName(user) ? user.email : user.id;
}

export function userInitials(user) {
  const name = userName(user);
  return name
    .split(/[\s@._-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase())
    .join('') || 'U';
}

export function userMatches(user, query) {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;

  return [user.displayName, user.username, user.email, user.id]
    .filter(Boolean)
    .some((value) => value.toLowerCase().includes(needle));
}

export function selectedUserLabel(users, userId) {
  const user = users.find((candidate) => candidate.id === userId);
  return user ? userName(user) : 'selected user';
}
