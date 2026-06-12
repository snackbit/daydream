export function moveIdBefore(
  ids: string[],
  activeId: string,
  overId: string,
): string[] {
  const activeIndex = ids.indexOf(activeId);
  const overIndex = ids.indexOf(overId);
  if (activeIndex === -1 || overIndex === -1) {
    return ids;
  }

  const next = [...ids];
  const [active] = next.splice(activeIndex, 1);
  next.splice(overIndex, 0, active!);
  return next;
}
