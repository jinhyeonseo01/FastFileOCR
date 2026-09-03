export function selectIds(
  current: string[],
  id: string,
  visible: string[],
  anchor: string,
  shift: boolean,
  additive: boolean,
): string[] {
  if (shift) {
    const end = visible.indexOf(id);
    if (end < 0) return current;
    const found = visible.indexOf(anchor),
      start = found < 0 ? end : found;
    const range = visible.slice(Math.min(start, end), Math.max(start, end) + 1);
    return additive ? [...new Set([...current, ...range])] : range;
  }
  if (current.includes(id)) return current.filter((value) => value !== id);
  return additive ? [...current, id] : [id];
}
