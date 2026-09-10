import { ChangeSet } from "@codemirror/state";

export function contentChange(before: string, after: string) {
  let from = 0;
  while (from < before.length && from < after.length && before[from] === after[from]) from += 1;
  let oldEnd = before.length;
  let newEnd = after.length;
  while (oldEnd > from && newEnd > from && before[oldEnd - 1] === after[newEnd - 1]) {
    oldEnd -= 1;
    newEnd -= 1;
  }
  return { from, to: oldEnd, insert: after.slice(from, newEnd) };
}

export function contentChanges(before: string, after: string) {
  return ChangeSet.of(contentChange(before, after), before.length);
}

export function localDateTime(date = new Date()) {
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function articleFileName(title: string) {
  return title.trim().replace(/[<>:"/\\|?*]/g, "-");
}
