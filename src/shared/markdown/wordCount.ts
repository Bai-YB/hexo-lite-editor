export function countWords(content: string) {
  const body = content.replace(/^---[\s\S]*?---/, "");
  const chinese = body.match(/[\u3400-\u9fff]/g)?.length ?? 0;
  const words = body.match(/[A-Za-z0-9_]+(?:[-'][A-Za-z0-9_]+)*/g)?.length ?? 0;
  return chinese + words;
}

export function lineAt(content: string, offset: number) {
  const end = Math.min(Math.max(offset, 0), content.length);
  let line = 1;
  for (let index = 0; index < end; index += 1) {
    if (content.charCodeAt(index) === 10) line += 1;
  }
  return line;
}
